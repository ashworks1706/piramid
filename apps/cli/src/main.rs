//! The piramid binary: subcommand parsing, server startup, and the developer console.

use std::io::Write;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use clap::{Parser, Subcommand};
mod animation;
mod console;
mod support;
use piramid::config::StartupConfig;
use piramid::observability;
use piramid::state::AppState;
use piramid::{embeddings, http};
use tokio::runtime::Runtime;

#[derive(Parser)]
#[command(author, version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the server in the foreground, with no terminal UI
    Serve {
        /// Config file to load
        #[arg(long)]
        config: Option<PathBuf>,
        /// Port to bind
        #[arg(long)]
        port: Option<u16>,
        /// Data directory
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },

    /// Write a diagnostic bundle to attach to a bug report. Secrets are redacted, review it first
    SupportBundle {
        /// Where to write the bundle
        #[arg(long, short, default_value = "piramid-support-bundle.md")]
        output: PathBuf,
        /// Config file to load
        #[arg(long)]
        config: Option<PathBuf>,
        /// Data directory to inspect
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },
}

/// Run action, printing context and exiting 1 on failure.
fn run_or_exit(action: impl FnOnce() -> std::io::Result<()>, context: &str) {
    if let Err(e) = action() {
        eprintln!("{context}: {e}");
        std::process::exit(1);
    }
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::SupportBundle {
            output,
            config,
            data_dir,
        }) => {
            run_or_exit(
                || support_bundle(output, config, data_dir),
                "Failed to write support bundle",
            );
        }
        Some(Commands::Serve {
            config,
            port,
            data_dir,
        }) => {
            animate();
            let source = config_source(config, port, data_dir);
            let config =
                piramid::config::loader::load_from(&source).unwrap_or_else(exit_on_config_error);
            run_or_exit(|| start_server_inline(config, source), "piramid serve");
        }
        // No subcommand opens the console. Inside a checkout it can drive the repo as well as
        // the server; an installed binary gets the views that need only a server.
        None => {
            let config = piramid::config::loader::load().unwrap_or_else(exit_on_config_error);
            let cwd = match std::env::current_dir() {
                Ok(cwd) => cwd,
                Err(e) => {
                    eprintln!("piramid: cannot read the working directory: {e}");
                    std::process::exit(1);
                }
            };
            let (profile, root) = match console::repo_root(&cwd) {
                Some(root) => (console::Profile::Developer, root),
                None => (console::Profile::Production, cwd),
            };
            run_or_exit(
                || console::run(&config, profile, root),
                "Failed to start the console",
            );
        }
    }
}

/// Report a configuration failure and exit.
fn exit_on_config_error<T>(error: piramid::error::ConfigError) -> T {
    eprintln!("piramid: {error}");
    std::process::exit(1);
}

/// The configuration source the command-line flags name.
fn config_source(
    file: Option<PathBuf>,
    port: Option<u16>,
    data_dir: Option<PathBuf>,
) -> piramid::config::loader::ConfigSource {
    piramid::config::loader::ConfigSource {
        file,
        port,
        data_dir: data_dir.map(|dir| dir.to_string_lossy().into_owned()),
    }
}

fn support_bundle(
    output: PathBuf,
    config: Option<PathBuf>,
    data_dir: Option<PathBuf>,
) -> std::io::Result<()> {
    let source = config_source(config, None, data_dir);
    let config = piramid::config::loader::load_from(&source).unwrap_or_else(exit_on_config_error);
    let state = std::sync::Arc::new(
        AppState::new(config.clone(), embeddings::EmbeddingsManager::disabled())
            .map_err(std::io::Error::other)?,
    );
    let failed = preload_collections_for_metrics(&state)?;

    let bundle = support::Bundle {
        config: &config,
        config_file: source.file.as_deref(),
        state: &state,
        failed_collections: &failed,
    };
    support::write(&bundle, &output)?;
    println!("wrote {}", output.display());
    println!("Review it before sharing — it contains your configuration and collection names.");
    Ok(())
}

/// Open every collection on disk and return the name and error of each one that fails to open.
fn preload_collections_for_metrics(
    state: &std::sync::Arc<AppState>,
) -> std::io::Result<Vec<(String, String)>> {
    let names = state
        .collection_manager
        .discover_on_disk()
        .map_err(std::io::Error::other)?;
    let mut failed = Vec::new();
    for collection_name in names {
        if let Err(error) = state.get_existing_collection(&collection_name) {
            eprintln!("Collection '{collection_name}' failed to open: {error}");
            failed.push((collection_name, error.to_string()));
        }
    }
    Ok(failed)
}

fn start_server_inline(
    config: piramid::config::Config,
    source: piramid::config::loader::ConfigSource,
) -> std::io::Result<()> {
    let rt = Runtime::new().map_err(std::io::Error::other)?;
    rt.block_on(async {
        let _observability =
            observability::install(config.startup.logging, &config.startup.telemetry)
                .map_err(std::io::Error::other)?;
        init_thread_pool(&config.startup)?;
        if config.startup.logging.config {
            tracing::info!(
                target: "piramid::config",
                config = ?config,
                "using_configuration"
            );
        }

        let embeddings = match &config.startup.embedding {
            Some(embedding) => {
                embeddings::EmbeddingsManager::from_config(embedding).map_err(|e| {
                    std::io::Error::other(format!(
                        "embedding provider configured but failed to initialize: {e}"
                    ))
                })?
            }
            None => embeddings::EmbeddingsManager::disabled(),
        };
        let gpu = if config.startup.hardware.gpu_enabled() {
            let hardware = config.startup.hardware;
            let manager = tokio::task::spawn_blocking(move || open_gpu(&hardware))
                .await
                .map_err(std::io::Error::other)??;
            Some(std::sync::Arc::new(manager))
        } else {
            None
        };
        let inference = if config.runtime.inference.enabled {
            let inference_config = config.runtime.inference.clone();
            let hardware = config.startup.hardware;
            let device = gpu.clone();
            let manager = tokio::task::spawn_blocking(move || {
                piramid::inference::InferenceManager::load(
                    &inference_config,
                    &hardware,
                    device.as_deref(),
                    std::sync::Arc::new(piramid::fusion::NoopRetrievalHook),
                )
            })
            .await
            .map_err(std::io::Error::other)?
            .map_err(|e| std::io::Error::other(format!("model failed to load: {e}")))?;
            Some(std::sync::Arc::new(manager))
        } else {
            None
        };
        let addr = config.startup.bind.clone();
        let data_dir = config.startup.data_dir.clone();
        let mut state = AppState::new(config, embeddings)
            .map_err(std::io::Error::other)?
            .with_config_source(source);
        if let Some(manager) = gpu {
            state = state.with_gpu(manager);
        }
        if let Some(manager) = inference {
            state = state
                .with_inference(manager)
                .map_err(std::io::Error::other)?;
        }
        let state = std::sync::Arc::new(state);

        tracing::info!(
            target: "piramid::config",
            address = addr.as_str(),
            data_dir = data_dir.as_str(),
            "server_starting"
        );
        let shutdown = shutdown_signal()?;
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .map_err(|e| std::io::Error::other(format!("bind {addr} failed: {e}")))?;
        http::serve::serve(state, listener, shutdown)
            .await
            .map_err(std::io::Error::other)
    })
}

/// Open the configured device with its memory budget and serve the gpu execution mode from it.
fn open_gpu(
    hardware: &piramid::config::HardwareConfig,
) -> std::io::Result<piramid::gpu::GpuManager> {
    let vram = hardware.vram;
    let settings = piramid::gpu::BudgetSettings {
        limit_bytes: hardware.gpu_memory_budget_bytes,
        reserve_bytes: hardware.gpu.reserve_bytes,
        shares: vram.enabled.then_some(piramid::gpu::PoolShares {
            weights: vram.weights_ratio,
            kv_cache: vram.kv_ratio,
            index: vram.index_ratio,
        }),
    };
    let manager =
        piramid::gpu::GpuManager::open(hardware.gpu.device_ordinal, settings, hardware.gpu.streams)
            .map_err(|e| std::io::Error::other(format!("startup.hardware: {e}")))?;
    install_gpu(&manager, hardware.gpu.distance_block_size)?;
    Ok(manager)
}

#[cfg(feature = "gpu-cuda")]
fn install_gpu(manager: &piramid::gpu::GpuManager, block_size: u32) -> std::io::Result<()> {
    piramid::compute::strategies::install_gpu(manager, block_size)
        .map_err(|e| std::io::Error::other(format!("startup.hardware.gpu: {e}")))
}

#[cfg(not(feature = "gpu-cuda"))]
fn install_gpu(_manager: &piramid::gpu::GpuManager, _block_size: u32) -> std::io::Result<()> {
    Err(std::io::Error::other(
        "startup.hardware.profile: gpu needs a build with the gpu-cuda feature",
    ))
}

/// A future that completes on the first SIGINT or SIGTERM.
///
/// Returns an error if a handler cannot be installed.
#[cfg(unix)]
fn shutdown_signal() -> std::io::Result<impl std::future::Future<Output = ()>> {
    use tokio::signal::unix::{signal, SignalKind};
    let mut interrupt = signal(SignalKind::interrupt())?;
    let mut terminate = signal(SignalKind::terminate())?;
    Ok(async move {
        let name = tokio::select! {
            _ = interrupt.recv() => "SIGINT",
            _ = terminate.recv() => "SIGTERM",
        };
        tracing::info!(target: "piramid::shutdown", signal = name, "shutdown_signal_received");
    })
}

/// A future that completes on the first Ctrl-C.
#[cfg(not(unix))]
fn shutdown_signal() -> std::io::Result<impl std::future::Future<Output = ()>> {
    Ok(async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!(target: "piramid::shutdown", %error, "shutdown_signal_failed");
        }
    })
}

/// Build the global rayon pool. Called once, before any collection opens.
///
/// Returns an error if the global pool cannot be built.
fn init_thread_pool(startup: &StartupConfig) -> std::io::Result<()> {
    rayon::ThreadPoolBuilder::new()
        .num_threads(startup.num_threads())
        .build_global()
        .map_err(|error| std::io::Error::other(format!("startup.threads: {error}")))
}

fn animate() {
    // The splash is skipped when stdout is not a terminal.
    if !std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        return;
    }
    print!("\x1b[2J\x1b[H\x1b[?25l");
    let _ = std::io::stdout().flush();

    for (i, frame) in animation::CLI_FRAMES.iter().enumerate() {
        print!("\x1b[H{frame}");
        let _ = std::io::stdout().flush();
        thread::sleep(Duration::from_millis(45));
        if i > 12 {
            break;
        }
    }

    print!("\x1b[2J\x1b[H\n\x1b[?25h");
    let _ = std::io::stdout().flush();
}
