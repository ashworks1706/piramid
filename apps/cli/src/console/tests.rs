//! Unit tests for the parts of the console that are not drawing.

use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::app::parse_command;
use super::logs::{LogBuffer, LogWriter};
use super::runner::{parse_ps, sanitize_line};
use super::settings::{repo_root, Settings};
use super::types::{Command, Group, LogLine, Profile, ServiceState, Status, Stream, View};
use super::units::catalog;

#[test]
fn commands_parse_into_actions() {
    assert_eq!(parse_command("q"), Command::Quit);
    assert_eq!(parse_command("start serve"), Command::Start("serve".into()));
    assert_eq!(
        parse_command("stop  ollama"),
        Command::Stop("ollama".into())
    );
    assert_eq!(parse_command("restart web"), Command::Restart("web".into()));
    assert_eq!(parse_command("help"), Command::Help);
    assert_eq!(parse_command("clear"), Command::Clear);
}

#[test]
fn an_unrecognised_word_is_handed_to_just() {
    // An unrecognised head word is passed through to just.
    assert_eq!(
        parse_command("check-gpu"),
        Command::Just(vec!["check-gpu".into()])
    );
    assert_eq!(
        parse_command("bench --save-baseline main"),
        Command::Just(vec![
            "bench".into(),
            "--save-baseline".into(),
            "main".into()
        ])
    );
    // The start word with no argument is a recipe name, not a malformed start.
    assert_eq!(parse_command("start"), Command::Just(vec!["start".into()]));
    assert_eq!(parse_command("   "), Command::Unknown(String::new()));
}

#[test]
fn the_catalog_is_unique_and_every_unit_is_runnable() {
    let units = catalog();
    let ids: std::collections::HashSet<&str> = units.iter().map(|u| u.id.as_str()).collect();
    assert_eq!(ids.len(), units.len(), "two units share an id");
    // Every unit is either a compose service or a just recipe.
    assert!(units
        .iter()
        .all(|u| u.service().is_some() || !u.args.is_empty()));
    assert!(units.iter().any(|u| u.id == "serve"));
    // A named task keeps its name rather than its command line.
    let bundle = units
        .iter()
        .find(|u| u.id == "support-bundle")
        .expect("the catalog offers a support bundle");
    assert_eq!(bundle.args, ["piramid", "support-bundle"]);
    // The resolved configuration is a view now, not a recipe to shell out to.
    assert!(!units.iter().any(|u| u.id == "config"));
    assert!(
        !units.iter().any(|u| u.id.starts_with("piramid ")),
        "a unit is showing its command line as its name"
    );
    assert!(units.iter().any(|u| u.id == "check"));
    assert!(units
        .iter()
        .any(|u| u.id == "prod-up" && u.group == Group::Deploy));
    assert!(units
        .iter()
        .any(|u| u.id == "ollama" && u.group == Group::Containers));
}

#[test]
fn every_catalog_recipe_exists_in_the_justfile() {
    let Some(root) = repo_root(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))) else {
        return;
    };
    let justfile = std::fs::read_to_string(root.join("justfile")).unwrap_or_default();
    let recipes: std::collections::HashSet<String> = justfile
        .lines()
        .filter(|line| !line.starts_with(char::is_whitespace) && line.contains(':'))
        .filter_map(|line| line.split([':', ' ']).next())
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect();

    for unit in catalog() {
        let Some(recipe) = unit.args.first() else {
            continue;
        };
        assert!(
            recipes.contains(recipe),
            "catalog runs `just {recipe}`, which the justfile does not define"
        );
    }
}

/// A console over a scratch directory, with no repo behind it.
fn console() -> super::app::App {
    let root = std::env::temp_dir().join(format!("piramid-console-{}", std::process::id()));
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let config = piramid_core::config::Config::default();
    super::app::App::new(
        Settings::from_config(&config),
        Profile::Developer,
        root,
        &tx,
    )
    .expect("the log directory is creatable")
}

fn press(key: char) -> super::types::Event {
    super::types::Event::Key(KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE))
}

#[test]
fn navigation_moves_through_the_catalog_and_stops_at_both_ends() {
    let mut app = console();
    assert_eq!(app.current().unit.id, "serve");

    app.handle(press('k'));
    assert_eq!(app.current().unit.id, "serve", "k at the top must not wrap");

    app.handle(press('j'));
    assert_eq!(app.current().unit.id, "web");

    app.handle(press('G'));
    let last = app.units.last().map(|state| state.unit.id.clone());
    assert_eq!(Some(app.current().unit.id.clone()), last);

    app.handle(press('j'));
    assert_eq!(
        Some(app.current().unit.id.clone()),
        last,
        "j at the end must not wrap"
    );

    app.handle(press('g'));
    app.handle(press('g'));
    assert_eq!(app.current().unit.id, "serve");
}

#[test]
fn help_swallows_the_next_key_rather_than_acting_on_it() {
    let mut app = console();
    app.handle(press('?'));
    assert!(app.help);

    // The key that closes help must not also start whatever is selected.
    app.handle(press('\r'));
    assert!(!app.help);
    assert_eq!(app.current().status, Status::Stopped);
}

#[test]
fn quitting_is_q_or_ctrl_c() {
    let mut app = console();
    app.handle(press('q'));
    assert!(app.should_quit);

    let mut app = console();
    app.handle(super::types::Event::Key(KeyEvent::new(
        KeyCode::Char('c'),
        KeyModifiers::CONTROL,
    )));
    assert!(app.should_quit);
}

#[test]
fn stopping_something_that_is_not_running_says_so_instead_of_signalling() {
    let mut app = console();
    app.handle(press('x'));
    assert_eq!(app.notice.as_deref(), Some("serve is not running"));
}

#[test]
fn compose_states_map_onto_statuses() {
    let state = |state: &str, health: &str, exit_code| ServiceState {
        state: state.into(),
        health: health.into(),
        exit_code,
    };
    assert_eq!(state("running", "healthy", 0).status(), Status::Running);
    assert_eq!(state("running", "", 0).status(), Status::Running);
    assert_eq!(state("running", "starting", 0).status(), Status::Starting);
    assert_eq!(
        state("running", "unhealthy", 0).status(),
        Status::Failed("unhealthy".into())
    );
    assert_eq!(state("exited", "", 0).status(), Status::Stopped);
    assert_eq!(state("exited", "", 137).status(), Status::Exited(137));
}

#[test]
fn ps_output_parses_as_an_array_or_as_lines() {
    let array = r#"[{"Service":"piramid","State":"running","Health":"healthy","ExitCode":0}]"#;
    assert_eq!(
        parse_ps(array).unwrap_or_default()["piramid"].health,
        "healthy"
    );
    let lines = "{\"Service\":\"piramid\",\"State\":\"exited\",\"ExitCode\":1}\n{\"Service\":\"ollama\",\"State\":\"running\"}\n";
    let parsed: HashMap<_, _> = parse_ps(lines).unwrap_or_default();
    assert_eq!(parsed["piramid"].exit_code, 1);
    assert_eq!(parsed["ollama"].status(), Status::Running);
    assert!(parse_ps("").is_ok_and(|m| m.is_empty()));
    // A failed query returns an error rather than an empty set of services.
    assert!(parse_ps("not json").is_err());
}

#[test]
fn a_log_line_cannot_move_the_cursor_out_of_its_pane() {
    assert_eq!(
        sanitize_line("\x1b[32m   Compiling\x1b[0m piramid"),
        "   Compiling piramid"
    );
    assert_eq!(sanitize_line("plain"), "plain");
    // Carriage returns in compose progress lines are stripped.
    assert_eq!(
        sanitize_line("Container deploy-piramid-1  Recreated\r"),
        "Container deploy-piramid-1  Recreated"
    );
    assert_eq!(sanitize_line("a\rb\x08c\x07"), "abc");
    assert_eq!(sanitize_line("keeps\ttabs"), "keeps\ttabs");
}

#[test]
fn the_log_buffer_drops_the_oldest_line_and_searches_wrapping() {
    let mut buffer = LogBuffer::new(3);
    for text in ["alpha", "Beta", "gamma", "delta"] {
        buffer.push(LogLine::now(Stream::Out, text));
    }
    let texts: Vec<&str> = buffer.lines().map(|line| line.text.as_str()).collect();
    assert_eq!(texts, ["Beta", "gamma", "delta"]);
    assert_eq!(buffer.find("beta", 2, false), Some(0));
    assert_eq!(buffer.find("delta", 0, true), Some(2));
    assert_eq!(buffer.find("zeta", 0, false), None);
    assert_eq!(buffer.find("", 0, false), None);
}

#[test]
fn full_output_is_kept_on_disk_after_the_pane_scrolls_past_it() {
    // Unit tests get their scratch directory from the system temp dir.
    let dir = std::env::temp_dir().join(format!("piramid-console-logs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut writer = LogWriter::new(&dir).expect("the log directory is creatable");

    writer
        .append("serve", &LogLine::now(Stream::Out, "listening"))
        .expect("a line is writable");
    // The id of an ad-hoc task is a whole command line, which is not a filename.
    writer
        .append(
            "bench --save-baseline main",
            &LogLine::now(Stream::Out, "done"),
        )
        .expect("a line is writable");

    let saved = std::fs::read_to_string(dir.join("serve.log")).unwrap_or_default();
    assert!(saved.ends_with(" out listening\n"), "got {saved:?}");
    assert!(dir.join("bench---save-baseline-main.log").is_file());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn console_settings_come_from_the_one_configuration_file() {
    let config = piramid_core::config::Config::default();
    let settings = Settings::from_config(&config);

    // Unset, the console follows the address the server in the same file binds, so a deployment
    // that moves the port does not have to say so twice.
    assert_eq!(config.console.base_url, "");
    assert_eq!(settings.base_url, "http://127.0.0.1:6333");
    assert_eq!(settings.web_url, "http://localhost:3000");
    assert_eq!(
        settings.log_dir_under(std::path::Path::new("/repo")),
        std::path::Path::new("/repo/target/console-logs")
    );

    let mut moved = piramid_core::config::Config::default();
    moved.startup.bind = "0.0.0.0:7000".to_owned();
    assert_eq!(
        Settings::from_config(&moved).base_url,
        "http://localhost:7000"
    );
}

#[test]
fn the_repo_root_is_found_from_a_nested_directory() {
    let here = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = repo_root(here).unwrap_or_default();
    assert!(root.join("justfile").is_file());
    // An installed binary run from outside a checkout finds nothing, and prints help instead.
    assert!(repo_root(std::path::Path::new("/")).is_none());
}

#[test]
fn a_production_console_hides_the_views_that_need_a_checkout() {
    // The units view drives just recipes and compose, neither of which exists outside a checkout.
    assert_eq!(
        Profile::Production.views(),
        [View::Collections, View::Config, View::Device]
    );
    assert_eq!(
        Profile::Developer.views(),
        [View::Units, View::Collections, View::Config, View::Device]
    );
    // The first view is what the console opens on, and every profile has one.
    assert!(!Profile::Production.views().is_empty());
    assert!(!Profile::Developer.views().is_empty());
}

#[test]
fn a_digit_switches_view_and_an_absent_one_says_so() {
    let mut app = console();
    assert_eq!(app.view, View::Units);

    app.handle(press('2'));
    assert_eq!(app.view, View::Collections);
    app.handle(press('3'));
    assert_eq!(app.view, View::Config);
    app.handle(press('1'));
    assert_eq!(app.view, View::Units);

    app.handle(press('9'));
    assert_eq!(app.view, View::Units, "an absent view must not switch");
    assert_eq!(app.notice.as_deref(), Some("no view 9"));
}

#[test]
fn keys_reach_the_view_that_is_showing() {
    let mut app = console();

    // j moves the unit selection while units is showing.
    app.handle(press('j'));
    assert_eq!(app.current().unit.id, "web");

    // On the config view the same key scrolls instead, and leaves the unit selection alone.
    app.handle(press('3'));
    app.handle(press('j'));
    assert_eq!(app.config_scroll, 1);
    assert_eq!(app.current().unit.id, "web");
}

#[test]
fn an_unreachable_server_is_reported_rather_than_left_blank() {
    use super::client::ClientError;

    let mut app = console();
    app.collections.snapshot(Err(ClientError::Unreachable(
        "/api/metrics".into(),
        "Connection refused".into(),
    )));

    // The status bar and the empty list both read this, so a console pointed at nothing says so
    // instead of waiting forever.
    assert!(app.collections.error.is_some());
    assert!(app.collections.rows.is_empty());
}

/// A console watching the server at base_url.
fn console_watching(base_url: &str) -> super::app::App {
    let root = std::env::temp_dir().join(format!("piramid-console-{}", std::process::id()));
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let mut config = piramid_core::config::Config::default();
    config.console.base_url = base_url.to_owned();
    super::app::App::new(
        Settings::from_config(&config),
        Profile::Production,
        root,
        &tx,
    )
    .expect("the log directory is creatable")
}

#[test]
fn loopback_urls_are_this_machine_and_every_other_host_is_not() {
    use super::device::is_loopback;

    for local in [
        "http://localhost:6333",
        "http://LOCALHOST",
        "http://127.0.0.1:7000/",
        "http://127.3.2.1",
        "http://[::1]:6333/api",
        "http://user:secret@localhost:6333",
    ] {
        assert!(is_loopback(local), "{local} is this machine");
    }
    for remote in [
        "https://piramid.internal:6333",
        "http://10.0.0.5:6333",
        "http://[2001:db8::1]:6333",
        "http://localhost.example.com",
        "http://0.0.0.0:6333",
    ] {
        assert!(!is_loopback(remote), "{remote} is not this machine");
    }
}

/// A scratch directory holding an executable and a non-executable file.
#[cfg(unix)]
fn scratch_path(name: &str) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let dir = std::env::temp_dir().join(format!("piramid-console-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("the scratch directory is creatable");
    let tool = dir.join("htop");
    std::fs::write(&tool, "#!/bin/sh\n").expect("the stand-in is writable");
    std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755))
        .expect("the stand-in is executable");
    let plain = dir.join("nvtop");
    std::fs::write(&plain, "").expect("the stand-in is writable");
    std::fs::set_permissions(&plain, std::fs::Permissions::from_mode(0o644))
        .expect("the stand-in is not executable");
    dir
}

#[cfg(unix)]
#[test]
fn a_program_is_found_only_as_an_executable_on_path() {
    use super::device::find_program;

    let dir = scratch_path("find");
    let path = std::env::join_paths([std::path::Path::new("/nonexistent"), dir.as_path()])
        .expect("the path joins");
    assert_eq!(find_program("htop", Some(&path)), Some(dir.join("htop")));
    assert_eq!(find_program("nvtop", Some(&path)), None);
    assert_eq!(find_program("top-of-nothing", Some(&path)), None);
    assert_eq!(find_program("htop", None), None);
}

#[cfg(unix)]
#[test]
fn a_handoff_needs_a_local_server_and_an_installed_monitor() {
    use super::device::{DeviceView, Monitor};

    let dir = scratch_path("handoff");
    let path = dir.clone().into_os_string();

    let local = DeviceView::new("http://localhost:6333");
    assert_eq!(
        local.handoff(Monitor::Htop, Some(&path)),
        Ok(dir.join("htop"))
    );
    assert_eq!(
        local.handoff(Monitor::Nvtop, Some(&path)),
        Err("nvtop is not installed: not found on PATH".to_owned())
    );

    // The remote refusal wins even where the monitor is installed.
    let remote = DeviceView::new("https://piramid.internal:6333");
    assert_eq!(
        remote.handoff(Monitor::Htop, Some(&path)),
        Err(
            "htop shows this machine, and the console watches https://piramid.internal:6333"
                .to_owned()
        )
    );
}

#[test]
fn a_remote_console_says_why_it_will_not_open_htop() {
    let mut app = console_watching("https://piramid.internal:6333");
    app.handle(press('3'));
    assert_eq!(app.view, View::Device);

    app.handle(press('h'));
    assert!(app.handoff.is_none());
    assert_eq!(
        app.notice.as_deref(),
        Some("htop shows this machine, and the console watches https://piramid.internal:6333")
    );
}

#[test]
fn an_absent_reading_is_a_gap_in_the_graph_and_never_zero() {
    use super::client::HostMetrics;
    use super::device::DeviceView;
    use std::time::{Duration, Instant};

    let start = Instant::now();
    let reading = |cpu: Option<f32>| HostMetrics {
        cpu_percent: cpu,
        ..HostMetrics::default()
    };
    let mut view = DeviceView::new("http://localhost:6333");
    view.record(start, reading(Some(10.0)));
    view.record(start + Duration::from_secs(1), reading(Some(20.0)));
    view.record(start + Duration::from_secs(2), reading(None));
    view.record(start + Duration::from_secs(3), reading(Some(30.0)));

    let now = start + Duration::from_secs(3);
    let runs = view.series(now, |h| h.cpu_percent.map(f64::from));
    assert_eq!(
        runs,
        vec![vec![(-3.0, 10.0), (-2.0, 20.0)], vec![(-0.0, 30.0)]]
    );
    assert!(view
        .series(now, |h| h.memory_used_bytes.map(|b| b as f64))
        .is_empty());
}

#[test]
fn a_failed_refresh_records_a_sample_with_nothing_measured() {
    use super::client::{ClientError, Metrics, Snapshot};

    let mut app = console();
    let mut metrics = Metrics::default();
    metrics.host.cpu_percent = Some(42.0);
    app.handle(super::types::Event::Snapshot(Box::new(Ok(Snapshot {
        metrics,
        ..Snapshot::default()
    }))));
    assert_eq!(app.device.latest().and_then(|h| h.cpu_percent), Some(42.0));

    app.handle(super::types::Event::Snapshot(Box::new(Err(
        ClientError::Unreachable("/api/metrics".into(), "Connection refused".into()),
    ))));
    assert_eq!(app.device.samples.len(), 2);
    let latest = app.device.latest().expect("a sample was recorded");
    assert_eq!(latest.cpu_percent, None);
    assert_eq!(latest.memory_total_bytes, None);
}

#[test]
fn host_fields_the_server_leaves_out_read_as_absent() {
    let metrics: super::client::Metrics =
        serde_json::from_str(r#"{"host": {"memory_total_bytes": 4096, "cpu_percent": 0.0}}"#)
            .expect("the body decodes");
    assert_eq!(metrics.host.memory_total_bytes, Some(4096));
    assert_eq!(metrics.host.cpu_percent, Some(0.0));
    assert_eq!(metrics.host.memory_used_bytes, None);
    assert_eq!(metrics.host.process_resident_bytes, None);

    let older: super::client::Metrics = serde_json::from_str("{}").expect("the body decodes");
    assert_eq!(older.host.cpu_percent, None);
}

type ServeTask = tokio::task::JoinHandle<Result<(), piramid_serving::http::serve::ServeError>>;

/// A server on a loopback port that requires key, the sender that stops it, and its task.
async fn server_requiring(
    key: &str,
    name: &str,
) -> (String, tokio::sync::oneshot::Sender<()>, ServeTask) {
    let dir = std::env::temp_dir().join(format!("piramid-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut config = piramid_core::config::Config::default();
    config.startup.data_dir = dir.to_string_lossy().into_owned();
    config.startup.http.auth.api_key =
        Some(piramid_core::config::ApiKey::new(key.to_owned()).unwrap());
    let state = std::sync::Arc::new(
        piramid_serving::state::AppState::new(
            config,
            piramid_model::embeddings::EmbeddingsManager::disabled(),
        )
        .unwrap(),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let task = tokio::spawn(piramid_serving::http::serve::serve(
        state,
        listener,
        async move {
            let _ = rx.await;
        },
    ));
    (base, tx, task)
}

#[tokio::test]
async fn a_rejected_key_is_reported_as_an_authentication_failure_not_as_unreachable() {
    use super::client::{Client, ClientError};
    use piramid_core::config::ApiKey;

    let (base, stop, task) = server_requiring("console-test-key", "console_auth").await;
    let timeout = std::time::Duration::from_secs(5);

    let missing = Client::new(&base, timeout, None).unwrap();
    let error = missing.snapshot().await.unwrap_err();
    assert!(
        matches!(error, ClientError::Unauthorized { .. }),
        "{error:?}"
    );
    assert!(error.to_string().contains("PIRAMID_API_KEY"), "{error}");

    let wrong = Client::new(&base, timeout, Some(ApiKey::new("nope".into()).unwrap())).unwrap();
    assert!(matches!(
        wrong.snapshot().await.unwrap_err(),
        ClientError::Unauthorized { .. }
    ));

    let right = Client::new(
        &base,
        timeout,
        Some(ApiKey::new("console-test-key".into()).unwrap()),
    )
    .unwrap();
    right.snapshot().await.unwrap();
    right.config().await.unwrap();

    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
    let dir = std::env::temp_dir().join(format!("piramid-console_auth-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_console_sends_the_key_the_environment_set() {
    let mut config = piramid_core::config::Config::default();
    assert!(Settings::from_config(&config).api_key.is_none());

    let key = piramid_core::config::ApiKey::new("from-env".into()).unwrap();
    config.startup.http.auth.api_key = Some(key.clone());
    assert_eq!(Settings::from_config(&config).api_key, Some(key));
}
