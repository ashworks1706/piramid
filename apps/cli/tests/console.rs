//! Tests for the parts of the console that are not drawing.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]

use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use piramid::console::app::parse_command;
use piramid::console::logs::{LogBuffer, LogWriter};
use piramid::console::runner::{parse_ps, sanitize_line};
use piramid::console::settings::{repo_root, Settings};
use piramid::console::types::{
    Command, Group, LogLine, Profile, ServiceState, Status, Stream, View,
};
use piramid::console::units::catalog;

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
    assert_eq!(
        parse_command("   "),
        Command::Unknown("empty command".into())
    );
}

#[test]
fn a_unit_command_without_a_unit_is_refused() {
    for word in ["start", "stop", "restart"] {
        assert_eq!(
            parse_command(word),
            Command::Unknown(format!("{word} needs a unit name"))
        );
    }
}

#[test]
fn each_command_has_one_spelling() {
    assert_eq!(parse_command("quit"), Command::Just(vec!["quit".into()]));
    assert_eq!(parse_command("h"), Command::Just(vec!["h".into()]));
    assert_eq!(
        parse_command("just check"),
        Command::Just(vec!["just".into(), "check".into()])
    );
}

#[test]
fn the_catalog_is_unique_and_every_unit_is_runnable() {
    let units = catalog("http://localhost:6333");
    let ids: std::collections::HashSet<&str> = units.iter().map(|u| u.id.as_str()).collect();
    assert_eq!(ids.len(), units.len(), "two units share an id");
    // Every unit is either a compose service or a just recipe.
    assert!(units
        .iter()
        .all(|u| u.service().is_some() || !u.args.is_empty()));
    assert!(units.iter().any(|u| u.id == "serve"));
    // A named task is identified by its name, not its command line.
    let bundle = units
        .iter()
        .find(|u| u.id == "support-bundle")
        .expect("the catalog offers a support bundle");
    assert_eq!(bundle.args, ["piramid", "support-bundle"]);
    // The resolved configuration is a view, not a recipe.
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
    let root = repo_root(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
        .expect("the crate is built from a checkout with a justfile at its root");
    let justfile =
        std::fs::read_to_string(root.join("justfile")).expect("the justfile is readable");
    let recipes: std::collections::HashSet<String> = justfile
        .lines()
        .filter(|line| !line.starts_with(char::is_whitespace) && line.contains(':'))
        .filter_map(|line| line.split([':', ' ']).next())
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect();

    for unit in catalog("http://localhost:6333") {
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
fn console() -> piramid::console::app::App {
    let root = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("piramid-console-{}", std::process::id()));
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let config = piramid_core::config::Config::default();
    piramid::console::app::App::new(
        Settings::from_config(&config).unwrap(),
        Profile::Developer,
        root,
        &tx,
    )
    .expect("the log directory is creatable")
}

fn press(key: char) -> piramid::console::types::Event {
    piramid::console::types::Event::Key(KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE))
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
    app.handle(piramid::console::types::Event::Key(KeyEvent::new(
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
    assert_eq!(parse_ps(array).unwrap()["piramid"].health, "healthy");
    let lines = "{\"Service\":\"piramid\",\"State\":\"exited\",\"Health\":\"\",\"ExitCode\":1}\n{\"Service\":\"ollama\",\"State\":\"running\",\"Health\":\"\",\"ExitCode\":0}\n";
    let parsed: HashMap<_, _> = parse_ps(lines).unwrap();
    assert_eq!(parsed["piramid"].exit_code, 1);
    assert_eq!(parsed["ollama"].status(), Status::Running);
    assert!(parse_ps("").is_ok_and(|m| m.is_empty()));
    // A failed query returns an error.
    assert!(parse_ps("not json").is_err());
    // A row without an exit code is refused.
    assert!(parse_ps(r#"{"Service":"piramid","State":"exited","Health":""}"#).is_err());
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
    let mut buffer = LogBuffer::new(std::num::NonZeroUsize::new(3).unwrap());
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
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("piramid-console-logs-{}", std::process::id()));
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
    let settings = Settings::from_config(&config).unwrap();

    // Unset, the console follows the address the server in the same file binds.
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
        Settings::from_config(&moved).unwrap().base_url,
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
    use piramid::console::client::ClientError;

    let mut app = console();
    app.collections.snapshot(Err(ClientError::Unreachable(
        "/api/metrics".into(),
        "Connection refused".into(),
    )));

    // A refresh against an unreachable server records the error and leaves no rows.
    assert!(app.collections.error.is_some());
    assert!(app.collections.rows.is_empty());
}

/// A console watching the server at base_url.
fn console_watching(base_url: &str) -> piramid::console::app::App {
    let root = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("piramid-console-{}", std::process::id()));
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let mut config = piramid_core::config::Config::default();
    config.console.base_url = base_url.to_owned();
    piramid::console::app::App::new(
        Settings::from_config(&config).unwrap(),
        Profile::Production,
        root,
        &tx,
    )
    .expect("the log directory is creatable")
}

#[test]
fn loopback_urls_are_this_machine_and_every_other_host_is_not() {
    use piramid::console::device::is_loopback;

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

    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("piramid-console-{name}-{}", std::process::id()));
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
    use piramid::console::device::find_program;

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
    use piramid::console::device::{DeviceView, Monitor};

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

/// Host readings with only the processor reading set.
fn cpu_reading(cpu: Option<f32>) -> piramid::console::client::HostMetrics {
    piramid::console::client::HostMetrics {
        cpu_percent: cpu,
        memory_used_bytes: None,
        memory_total_bytes: None,
        process_cpu_percent: None,
        process_resident_bytes: None,
    }
}

#[test]
fn an_absent_reading_is_a_gap_in_the_graph_and_never_zero() {
    use piramid::console::device::DeviceView;
    use std::time::{Duration, Instant};

    let start = Instant::now();
    let mut view = DeviceView::new("http://localhost:6333");
    view.record(start, Some(cpu_reading(Some(10.0))), Vec::new(), None, None);
    view.record(
        start + Duration::from_secs(1),
        Some(cpu_reading(Some(20.0))),
        Vec::new(),
        None,
        None,
    );
    view.record(
        start + Duration::from_secs(2),
        Some(cpu_reading(None)),
        Vec::new(),
        None,
        None,
    );
    view.record(
        start + Duration::from_secs(3),
        Some(cpu_reading(Some(30.0))),
        Vec::new(),
        None,
        None,
    );

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

/// Readings of the GPU at index with only utilisation set.
fn busy_reading(index: u32, busy: Option<f32>) -> piramid::console::client::GpuMetrics {
    piramid::console::client::GpuMetrics {
        index,
        name: None,
        memory_used_bytes: None,
        memory_total_bytes: None,
        utilization_percent: busy,
        temperature_celsius: None,
    }
}

#[test]
fn an_absent_gpu_reading_is_a_gap_in_the_graph_and_never_zero() {
    use piramid::console::device::DeviceView;
    use std::time::{Duration, Instant};

    let start = Instant::now();
    let at = |secs| start + Duration::from_secs(secs);
    let mut view = DeviceView::new("http://localhost:6333");
    view.record(at(0), None, vec![busy_reading(0, Some(10.0))], None, None);
    view.record(at(1), None, vec![busy_reading(0, None)], None, None);
    view.record(at(2), None, vec![busy_reading(0, Some(20.0))], None, None);
    view.record(at(3), None, Vec::new(), None, None);
    view.record(
        at(4),
        None,
        vec![busy_reading(1, Some(90.0)), busy_reading(0, Some(30.0))],
        None,
        None,
    );

    let now = at(4);
    assert_eq!(
        view.gpu_series(now, 0, |g| g.utilization_percent.map(f64::from)),
        vec![vec![(-4.0, 10.0)], vec![(-2.0, 20.0)], vec![(-0.0, 30.0)]]
    );
    assert_eq!(
        view.gpu_series(now, 1, |g| g.utilization_percent.map(f64::from)),
        vec![vec![(-0.0, 90.0)]]
    );
    assert!(view
        .gpu_series(now, 0, |g| g.temperature_celsius.map(f64::from))
        .is_empty());
    assert_eq!(view.gpu_indices(), vec![0, 1]);
    assert_eq!(
        view.latest_gpu(1).and_then(|g| g.utilization_percent),
        Some(90.0)
    );
}

#[test]
fn a_server_without_gpu_readings_has_no_gpu_in_the_device_view() {
    let mut app = console();
    let snapshot = snapshot_from(
        &metrics_body(r#", "host": {"cpu_percent": 42.0}"#),
        READY_BODY,
    );
    assert!(snapshot.metrics.gpus.is_empty());
    app.handle(piramid::console::types::Event::Snapshot(Box::new(Ok(
        snapshot,
    ))));
    assert!(app.device.gpu_indices().is_empty());

    app.handle(press('4'));
    let drawn = screen(&mut app);
    assert!(drawn.contains("cpu  host 42.0%"), "{drawn}");
    assert!(!drawn.contains("gpu "), "{drawn}");
}

#[test]
fn gpu_readings_are_decoded_and_drawn_in_the_device_view() {
    let mut app = console();
    let snapshot = snapshot_from(
        &metrics_body(
            r#", "host": {"cpu_percent": 42.0},
            "gpus": [{"index": 0, "name": "Test GPU", "memory_used_bytes": 1024,
                      "temperature_celsius": 61.0}]"#,
        ),
        READY_BODY,
    );
    let gpu = snapshot
        .metrics
        .gpus
        .first()
        .cloned()
        .expect("one gpu was sent");
    assert_eq!(gpu.memory_used_bytes, Some(1024));
    assert_eq!(gpu.memory_total_bytes, None);
    assert_eq!(gpu.utilization_percent, None);
    app.handle(piramid::console::types::Event::Snapshot(Box::new(Ok(
        snapshot,
    ))));

    app.handle(press('4'));
    let drawn = screen(&mut app);
    assert!(drawn.contains("gpu 0 Test GPU"), "{drawn}");
    assert!(drawn.contains("busy not reported"), "{drawn}");
    assert!(drawn.contains("temperature 61 C"), "{drawn}");
}

/// Generation readings of a model on cuda:0 with only decode rate and time to first token set.
fn generation_reading(
    decode: Option<f32>,
    first_token: Option<f32>,
) -> piramid::console::client::InferenceMetrics {
    piramid::console::client::InferenceMetrics {
        model: "qwen3-0.6b".into(),
        device: "cuda:0".into(),
        avg_time_to_first_token_ms: first_token,
        decode_tokens_per_second: decode,
        preemptions: 0,
        queue_depth: 0,
        running: 0,
        last_batch_size: 0,
        kv_blocks_total: 100,
        kv_blocks_used: 0,
        kv_blocks_cached: 0,
        kv_evictions: 0,
        prefix_hit_rate: None,
    }
}

#[test]
fn an_absent_generation_average_is_a_gap_in_the_graph_and_never_zero() {
    use piramid::console::device::DeviceView;
    use std::time::{Duration, Instant};

    let start = Instant::now();
    let at = |secs| start + Duration::from_secs(secs);
    let mut view = DeviceView::new("http://localhost:6333");
    view.record(
        at(0),
        None,
        Vec::new(),
        None,
        Some(generation_reading(None, None)),
    );
    view.record(
        at(1),
        None,
        Vec::new(),
        None,
        Some(generation_reading(Some(40.0), Some(120.0))),
    );
    view.record(at(2), None, Vec::new(), None, None);
    view.record(
        at(3),
        None,
        Vec::new(),
        None,
        Some(generation_reading(Some(50.0), None)),
    );

    let now = at(3);
    assert_eq!(
        view.inference_series(now, |i| i.decode_tokens_per_second.map(f64::from)),
        vec![vec![(-2.0, 40.0)], vec![(-0.0, 50.0)]]
    );
    assert_eq!(
        view.inference_series(now, |i| i.avg_time_to_first_token_ms.map(f64::from)),
        vec![vec![(-2.0, 120.0)]]
    );
    assert_eq!(
        view.latest_inference()
            .and_then(|i| i.avg_time_to_first_token_ms),
        None
    );
}

#[test]
fn kv_bar_cells_split_the_width_and_always_fill_it() {
    use piramid::console::ui::kv_cells;

    assert_eq!(kv_cells(25, 25, 100, 40), [10, 10, 20]);
    assert_eq!(kv_cells(0, 0, 0, 40), [0, 0, 40]);
    assert_eq!(kv_cells(100, 0, 100, 40), [40, 0, 0]);
    assert_eq!(kv_cells(90, 90, 100, 40), [36, 4, 0]);
    assert_eq!(kv_cells(1, 1, 3, 0), [0, 0, 0]);
}

/// The inference block of a metrics body with averages left out as the server leaves them out.
const INFERENCE_BODY: &str = r#", "host": {"cpu_percent": 42.0},
    "inference": {
        "model": "qwen3-0.6b", "device": "cuda:0",
        "requests_admitted": 9, "requests_finished": 7, "requests_failed": 0,
        "prompt_tokens": 900, "cached_prompt_tokens": 300, "generated_tokens": 700,
        "decode_tokens_per_second": 38.5, "avg_decode_step_ms": 26.0,
        "preemptions": 1, "queue_depth": 2, "running": 4, "last_batch_size": 3,
        "kv_blocks_total": 1200, "kv_blocks_used": 300, "kv_blocks_cached": 60,
        "kv_evictions": 5, "prefix_hit_rate": 0.25
    }"#;

#[test]
fn inference_readings_decode_with_unmeasured_averages_absent() {
    use piramid::console::client::{parse, Metrics};

    let metrics: Metrics =
        parse("/api/metrics", &metrics_body(INFERENCE_BODY)).expect("the body decodes");
    let inference = metrics.inference.expect("the inference block was sent");
    assert_eq!(inference.model, "qwen3-0.6b");
    assert_eq!(inference.decode_tokens_per_second, Some(38.5));
    assert_eq!(inference.avg_time_to_first_token_ms, None);
    assert_eq!(inference.kv_blocks_cached, 60);
    assert_eq!(inference.prefix_hit_rate, Some(0.25));

    // A server with no model loaded leaves the block out.
    let idle: Metrics =
        parse("/api/metrics", &metrics_body(r#", "host": {}"#)).expect("the body decodes");
    assert!(idle.inference.is_none());

    // A counter the server always sends is required.
    let missing = metrics_body(INFERENCE_BODY).replace(r#""queue_depth": 2,"#, "");
    let error = parse::<Metrics>("/api/metrics", &missing).expect_err("the key is required");
    assert!(error.to_string().contains("queue_depth"), "{error}");
}

#[test]
fn a_server_with_no_model_loaded_draws_no_generation_panels() {
    let mut app = console();
    let snapshot = snapshot_from(
        &metrics_body(r#", "host": {"cpu_percent": 42.0}"#),
        READY_BODY,
    );
    app.handle(piramid::console::types::Event::Snapshot(Box::new(Ok(
        snapshot,
    ))));
    assert!(app.device.latest_inference().is_none());

    app.handle(press('4'));
    let drawn = screen_of(&mut app, 200, 40);
    assert!(drawn.contains("cpu  host 42.0%"), "{drawn}");
    assert!(!drawn.contains("generation"), "{drawn}");
    assert!(!drawn.contains("prefix hits"), "{drawn}");
}

#[test]
fn inference_readings_are_drawn_in_the_device_view() {
    let mut app = console();
    let snapshot = snapshot_from(&metrics_body(INFERENCE_BODY), READY_BODY);
    app.handle(piramid::console::types::Event::Snapshot(Box::new(Ok(
        snapshot,
    ))));

    app.handle(press('4'));
    for (width, height) in [(200, 40), (80, 44), (60, 30)] {
        let drawn = screen_of(&mut app, width, height);
        assert!(drawn.contains("decode 38.5 tok/s"), "{drawn}");
        assert!(drawn.contains("first token not reported"), "{drawn}");
        assert!(drawn.contains("qwen3-0.6b on cuda:0"), "{drawn}");
        assert!(
            drawn.contains("used 300  cached 60  free 840  of 1,200"),
            "{drawn}"
        );
        assert!(drawn.contains("prefix hits 25.0%  evictions 5"), "{drawn}");
        assert!(
            drawn.contains("queue 2  running 4  batch 3  preempted 1"),
            "{drawn}"
        );
    }
}

#[test]
fn stacked_bar_cells_split_the_width_and_always_fill_it() {
    use piramid::console::ui::stacked_cells;

    assert_eq!(stacked_cells(&[25, 25, 10], 100, 40), vec![10, 10, 4, 16]);
    assert_eq!(stacked_cells(&[5, 5, 5], 0, 40), vec![0, 0, 0, 40]);
    assert_eq!(stacked_cells(&[80, 80, 80], 100, 40), vec![32, 8, 0, 0]);
    assert_eq!(stacked_cells(&[50], 100, 0), vec![0, 0]);
    assert_eq!(stacked_cells(&[], 100, 10), vec![10]);
}

/// The device memory budget of a metrics body, shared or split.
fn budget_body(shared: bool) -> String {
    let (weights, kv, vectors) = if shared {
        (8_u64 << 30, 8_u64 << 30, 8_u64 << 30)
    } else {
        (4_u64 << 30, 3_u64 << 30, 1_u64 << 30)
    };
    format!(
        r#", "host": {{"cpu_percent": 42.0}},
        "gpu_budget": {{
            "usable_bytes": {usable}, "shared": {shared},
            "pools": [
                {{"pool": "weights", "capacity_bytes": {weights}, "used_bytes": {w_used}}},
                {{"pool": "kv_cache", "capacity_bytes": {kv}, "used_bytes": {k_used}}},
                {{"pool": "vectors", "capacity_bytes": {vectors}, "used_bytes": {v_used}}}
            ]
        }}"#,
        usable = 8_u64 << 30,
        w_used = 2_u64 << 30,
        k_used = 1_u64 << 30,
        v_used = 512_u64 << 20,
    )
}

#[test]
fn a_device_memory_budget_decodes_and_is_absent_without_a_gpu() {
    use piramid::console::client::{parse, Metrics};

    let metrics: Metrics =
        parse("/api/metrics", &metrics_body(&budget_body(false))).expect("the body decodes");
    let budget = metrics.gpu_budget.expect("the budget was sent");
    assert_eq!(budget.usable_bytes, 8 << 30);
    assert!(!budget.shared);
    let pools: Vec<(&str, u64, u64)> = budget
        .pools
        .iter()
        .map(|p| (p.pool.as_str(), p.capacity_bytes, p.used_bytes))
        .collect();
    assert_eq!(
        pools,
        vec![
            ("weights", 4 << 30, 2 << 30),
            ("kv_cache", 3 << 30, 1 << 30),
            ("vectors", 1 << 30, 512 << 20),
        ]
    );

    // A server with no GPU open leaves the block out.
    let idle: Metrics =
        parse("/api/metrics", &metrics_body(r#", "host": {}"#)).expect("the body decodes");
    assert!(idle.gpu_budget.is_none());
}

#[test]
fn a_server_without_a_budget_draws_no_device_memory_panel() {
    let mut app = console();
    let snapshot = snapshot_from(
        &metrics_body(r#", "host": {"cpu_percent": 42.0}"#),
        READY_BODY,
    );
    app.handle(piramid::console::types::Event::Snapshot(Box::new(Ok(
        snapshot,
    ))));
    assert!(app.device.latest_budget().is_none());

    app.handle(press('4'));
    let drawn = screen_of(&mut app, 200, 40);
    assert!(drawn.contains("cpu  host 42.0%"), "{drawn}");
    assert!(!drawn.contains("device memory"), "{drawn}");
}

#[test]
fn a_shared_budget_draws_total_use_and_each_pool() {
    let mut app = console();
    let snapshot = snapshot_from(&metrics_body(&budget_body(true)), READY_BODY);
    app.handle(piramid::console::types::Event::Snapshot(Box::new(Ok(
        snapshot,
    ))));

    app.handle(press('4'));
    for (width, height) in [(200, 40), (80, 44), (60, 30)] {
        let drawn = screen_of(&mut app, width, height);
        assert!(
            drawn.contains("device memory  shared  3.5 GB of 8.0 GB"),
            "{drawn}"
        );
        assert!(
            drawn.contains("weights 2.0 GB  kv cache 1.0 GB  vectors 512.0 MB"),
            "{drawn}"
        );
        assert!(drawn.contains("free 4.5 GB"), "{drawn}");
        assert!(
            drawn.contains("every pool draws from one budget"),
            "{drawn}"
        );
    }
}

#[test]
fn a_split_budget_draws_each_pool_against_its_capacity() {
    let mut app = console();
    let snapshot = snapshot_from(&metrics_body(&budget_body(false)), READY_BODY);
    app.handle(piramid::console::types::Event::Snapshot(Box::new(Ok(
        snapshot,
    ))));

    app.handle(press('4'));
    for (width, height) in [(200, 40), (80, 44), (60, 30)] {
        let drawn = screen_of(&mut app, width, height);
        assert!(
            drawn.contains("device memory  split  3.5 GB of 8.0 GB"),
            "{drawn}"
        );
        assert!(drawn.contains("2.0 GB of 4.0 GB"), "{drawn}");
        assert!(drawn.contains("1.0 GB of 3.0 GB"), "{drawn}");
        assert!(drawn.contains("512.0 MB of 1.0 GB"), "{drawn}");
        assert!(!drawn.contains("every pool draws"), "{drawn}");
    }
}

#[test]
fn a_budget_and_generation_panels_share_the_device_view() {
    let mut app = console();
    let body = format!(
        "{}{}",
        budget_body(true),
        INFERENCE_BODY.replacen(r#", "host": {"cpu_percent": 42.0},"#, ",", 1)
    );
    let snapshot = snapshot_from(&metrics_body(&body), READY_BODY);
    app.handle(piramid::console::types::Event::Snapshot(Box::new(Ok(
        snapshot,
    ))));

    app.handle(press('4'));
    for (width, height) in [(200, 40), (80, 50)] {
        let drawn = screen_of(&mut app, width, height);
        assert!(drawn.contains("device memory  shared"), "{drawn}");
        assert!(drawn.contains("qwen3-0.6b on cuda:0"), "{drawn}");
        assert!(drawn.contains("queue 2  running 4"), "{drawn}");
    }
}

/// A metrics body with one collection, the host block given, and the rest as the server sends it.
fn metrics_body(host: &str) -> String {
    format!(
        r#"{{
            "total_collections": 1,
            "total_vectors": 3,
            "collections": [{{
                "name": "docs", "vector_count": 3,
                "memory_usage_bytes": 64, "insert_latency_ms": null, "search_latency_ms": 1.5,
                "lock_read_ms": null, "lock_write_ms": null
            }}],
            "wal_stats": [{{
                "collection": "docs", "last_checkpoint": null,
                "checkpoint_age_secs": null, "wal_size_bytes": 12
            }}],
            "embedding": {{"requests": 0, "texts": 0, "total_tokens": 0}}
            {host}
        }}"#
    )
}

/// A readiness body with one loaded collection.
const READY_BODY: &str = r#"{
    "ok": true, "version": "0.2.0", "data_dir": "/data", "total_collections": 1,
    "loaded_collections": 1, "total_vectors": 3,
    "collections": [{"name": "docs", "loaded": true, "integrity_ok": true}]
}"#;

/// A collection list body with one euclidean collection of width 384.
const LIST_BODY: &str = r#"{
    "collections": [
        {"name": "docs", "count": 3, "created_at": 1, "updated_at": 2, "dimensions": 384,
         "metric": "euclidean"}
    ]
}"#;

/// A snapshot decoded from bodies shaped like the server's, with the list of LIST_BODY.
fn snapshot_from(metrics: &str, ready: &str) -> piramid::console::client::Snapshot {
    snapshot_with(metrics, ready, LIST_BODY)
}

/// A snapshot decoded from metrics, readiness and collection list bodies.
fn snapshot_with(metrics: &str, ready: &str, list: &str) -> piramid::console::client::Snapshot {
    use piramid::console::client::parse;
    piramid::console::client::Snapshot {
        metrics: parse("/api/metrics", metrics).expect("the metrics body decodes"),
        ready: parse("/api/readyz", ready).expect("the readiness body decodes"),
        list: parse("/api/collections", list).expect("the list body decodes"),
    }
}

#[test]
fn a_failed_refresh_records_a_sample_with_nothing_measured() {
    use piramid::console::client::ClientError;

    let mut app = console();
    let snapshot = snapshot_from(
        &metrics_body(r#", "host": {"cpu_percent": 42.0}"#),
        READY_BODY,
    );
    app.handle(piramid::console::types::Event::Snapshot(Box::new(Ok(
        snapshot,
    ))));
    assert_eq!(app.device.latest().and_then(|h| h.cpu_percent), Some(42.0));

    app.handle(piramid::console::types::Event::Snapshot(Box::new(Err(
        ClientError::Unreachable("/api/metrics".into(), "Connection refused".into()),
    ))));
    assert_eq!(app.device.samples.len(), 2);
    assert!(app.device.latest().is_none());
}

#[test]
fn host_fields_the_server_leaves_out_read_as_absent() {
    use piramid::console::client::{parse, Metrics};

    let metrics: Metrics = parse(
        "/api/metrics",
        &metrics_body(r#", "host": {"memory_total_bytes": 4096, "cpu_percent": 0.0}"#),
    )
    .expect("the body decodes");
    let host = metrics.host;
    assert_eq!(host.memory_total_bytes, Some(4096));
    assert_eq!(host.cpu_percent, Some(0.0));
    assert_eq!(host.memory_used_bytes, None);
    assert_eq!(host.process_resident_bytes, None);

    // The server always sends the host block, so a body without it is a decode error.
    let error = parse::<Metrics>("/api/metrics", &metrics_body(""))
        .expect_err("the host block is required");
    assert!(error.to_string().contains("host"), "{error}");
}

#[test]
fn a_body_missing_a_field_the_server_always_sends_is_a_decode_error() {
    use piramid::console::client::{
        parse, ClientError, CollectionHealth, Metrics, Readyz, Version,
    };

    // Readiness always sends loaded; without it the collection must not read as not loaded.
    let error = parse::<CollectionHealth>("/api/readyz", r#"{"name": "docs"}"#)
        .expect_err("loaded is required");
    assert!(matches!(error, ClientError::Decode(..)), "{error:?}");
    assert!(error.to_string().contains("loaded"), "{error}");

    assert!(parse::<Readyz>("/api/readyz", "{}").is_err());
    assert!(parse::<Version>("/api/version", "{}").is_err());
    assert!(parse::<Metrics>("/api/metrics", r#"{"collections": []}"#).is_err());

    // A null the server always sends is required as a key, not only as a value.
    let no_latency = metrics_body(r#", "host": {}"#).replace(r#""search_latency_ms": 1.5,"#, "");
    let error = parse::<Metrics>("/api/metrics", &no_latency).expect_err("the key is required");
    assert!(error.to_string().contains("search_latency_ms"), "{error}");

    // Fields the server leaves out when empty are optional.
    let version: Version =
        parse("/api/version", r#"{"version": "0.2.0"}"#).expect("the commit is optional");
    assert_eq!(version.version, "0.2.0");
}

/// The text of every cell of the console drawn at 200 by 20.
fn screen(app: &mut piramid::console::app::App) -> String {
    screen_of(app, 200, 20)
}

/// The text of every cell of the console drawn at width by height.
fn screen_of(app: &mut piramid::console::app::App, width: u16, height: u16) -> String {
    let backend = ratatui::backend::TestBackend::new(width, height);
    let mut terminal = ratatui::Terminal::new(backend).expect("a test terminal opens");
    terminal
        .draw(|frame| piramid::console::ui::draw(frame, app))
        .expect("the frame draws");
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

#[test]
fn a_decode_error_in_a_refresh_is_on_the_screen() {
    use piramid::console::client::{parse, Readyz};

    let mut app = console();
    app.handle(press('2'));
    let error = parse::<Readyz>("/api/readyz", r#"{"collections": [{"name": "docs"}]}"#)
        .expect_err("loaded is required");
    app.handle(piramid::console::types::Event::Snapshot(Box::new(Err(
        error,
    ))));

    let drawn = screen(&mut app);
    assert!(drawn.contains("/api/readyz"), "{drawn}");
    assert!(drawn.contains("missing field `loaded`"), "{drawn}");
}

#[test]
fn a_config_response_without_the_config_key_is_an_error() {
    use piramid::console::client::{render_config, ClientError};

    let error =
        render_config(&serde_json::json!({ "startup": {} })).expect_err("the key is required");
    assert!(matches!(error, ClientError::Decode(..)), "{error:?}");
    assert!(error.to_string().contains("app_config"), "{error}");

    let rendered = render_config(&serde_json::json!({ "app_config": { "startup": {} } }))
        .expect("the key is present");
    assert!(rendered.starts_with("startup:"), "{rendered}");
}

/// A console on the collections view holding the snapshot of one collection.
fn console_with_a_collection() -> piramid::console::app::App {
    let mut app = console();
    app.handle(press('2'));
    app.handle(piramid::console::types::Event::Snapshot(Box::new(Ok(
        snapshot_from(&metrics_body(r#", "host": {}"#), READY_BODY),
    ))));
    app
}

#[test]
fn collections_messages_reach_the_status_bar() {
    // With no collection there is nothing to act on, and the console says so.
    let mut app = console();
    app.handle(press('2'));
    app.handle(press('c'));
    assert_eq!(app.notice.as_deref(), Some("no collection selected"));
    assert!(screen(&mut app).contains("no collection selected"));

    let mut app = console_with_a_collection();
    app.handle(press('c'));
    assert!(app.collections.pending.is_some());
    app.handle(press('n'));
    assert!(app.collections.pending.is_none());
    assert!(app.pending_action.is_none());
    assert!(screen(&mut app).contains("cancelled"));

    app.handle(press('r'));
    assert!(
        app.collections.pending.is_none(),
        "r starts no action in the collections view"
    );
    app.handle(press('c'));
    app.handle(press('y'));
    assert_eq!(
        app.pending_action,
        Some(piramid::console::collections::Pending::Compact(
            "docs".into()
        ))
    );
    assert!(screen(&mut app).contains("compaction of docs running"));

    app.handle(piramid::console::types::Event::Acted(Err(
        "compaction of docs failed: /api/collections/docs/compact returned 500: boom".into(),
    )));
    assert!(screen(&mut app).contains("compaction of docs failed"));
    app.handle(piramid::console::types::Event::Acted(Ok(
        "compaction of docs done".into(),
    )));
    assert!(screen(&mut app).contains("compaction of docs done"));
}

#[test]
fn a_finished_compaction_reports_documents_and_reclaimed_bytes() {
    use piramid::console::client::{parse, Compacted};

    let compacted: Compacted = parse(
        "/api/collections/docs/compact",
        r#"{"documents": 1200, "bytes_before": 5347737, "bytes_after": 3355443, "latency_ms": 12.0}"#,
    )
    .expect("the body decodes");

    assert_eq!(
        piramid::console::collections::compacted_line("docs", &compacted),
        "compaction of docs: 1200 documents, 5.1 MB -> 3.2 MB"
    );
}

#[test]
fn a_probe_failure_shows_its_reason() {
    use piramid::console::types::{Health, Probe};

    let mut app = console();
    app.handle(piramid::console::types::Event::Health(Box::new(Health {
        live: Probe::Down("Connection refused".into()),
        ready: Probe::Down("Connection refused".into()),
        web: Probe::Degraded("502 Bad Gateway: upstream".into()),
    })));
    let drawn = screen(&mut app);
    assert!(drawn.contains("server: Connection refused"), "{drawn}");
    assert!(drawn.contains("web: 502 Bad Gateway"), "{drawn}");

    app.handle(piramid::console::types::Event::ProbesStopped(
        "health probes are off: http client: no TLS backend".into(),
    ));
    assert!(screen(&mut app).contains("health probes are off"));
}

#[test]
fn the_reload_key_asks_for_the_configuration_again() {
    use piramid::console::types::ConfigState;

    let mut app = console();
    app.handle(piramid::console::types::Event::Config(Ok(
        "startup: {}".into()
    )));
    assert_eq!(app.config, Some(ConfigState::Loaded("startup: {}".into())));
    app.handle(press('3'));
    app.handle(press('R'));
    // The loop fetches whenever the configuration is None.
    assert!(app.config.is_none());

    app.handle(piramid::console::types::Event::Config(Err(
        "/api/config: boom".into(),
    )));
    assert!(screen(&mut app).contains("/api/config: boom"));
}

type ServeTask = tokio::task::JoinHandle<Result<(), piramid_serving::http::serve::ServeError>>;

/// A server on a loopback port that requires key, the sender that stops it, and its task.
async fn server_requiring(
    key: &str,
    name: &str,
) -> (String, tokio::sync::oneshot::Sender<()>, ServeTask) {
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("piramid-{name}-{}", std::process::id()));
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
    use piramid::console::client::{Client, ClientError};
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
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("piramid-console_auth-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_console_sends_the_key_the_environment_set() {
    let mut config = piramid_core::config::Config::default();
    assert!(Settings::from_config(&config).unwrap().api_key.is_none());

    let key = piramid_core::config::ApiKey::new("from-env".into()).unwrap();
    config.startup.http.auth.api_key = Some(key.clone());
    assert_eq!(Settings::from_config(&config).unwrap().api_key, Some(key));
}

#[test]
fn a_server_with_no_model_loaded_gives_the_device_graphs_the_whole_view() {
    let mut app = console();
    let snapshot = snapshot_from(
        &metrics_body(r#", "host": {"cpu_percent": 42.0}"#),
        READY_BODY,
    );
    app.handle(piramid::console::types::Event::Snapshot(Box::new(Ok(
        snapshot,
    ))));

    app.handle(press('4'));
    let (width, height) = (200, 40);
    let drawn: Vec<char> = screen_of(&mut app, width, height).chars().collect();
    let last_body_row: String = drawn
        .chunks(usize::from(width))
        .nth(usize::from(height) - 2)
        .expect("the screen has that row")
        .iter()
        .collect();
    assert!(!last_body_row.trim().is_empty(), "{last_body_row:?}");
}

#[test]
fn a_unit_killed_by_a_signal_it_was_not_asked_for_is_a_failure() {
    let mut app = console();
    app.handle(piramid::console::types::Event::Exited {
        unit: "serve".into(),
        code: None,
    });
    assert_eq!(
        app.current().status,
        Status::Failed("killed by a signal".into())
    );

    app.handle(piramid::console::types::Event::Exited {
        unit: "serve".into(),
        code: Some(3),
    });
    assert_eq!(app.current().status, Status::Exited(3));
}

#[test]
fn stopping_a_unit_this_console_did_not_start_is_an_error() {
    use piramid::console::runner::Runner;
    use piramid::console::types::RunnerError;

    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let mut runner = Runner::new(std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")), tx);
    let serve = catalog("http://localhost:6333")
        .into_iter()
        .find(|unit| unit.id == "serve")
        .expect("the catalog has a serve unit");
    let error = runner.stop(&serve).expect_err("nothing was started");
    assert!(
        matches!(&error, RunnerError::NotTracked { unit } if unit == "serve"),
        "{error:?}"
    );
}

#[test]
fn an_error_body_is_read_from_the_error_key_only() {
    use piramid::console::client::summarize;

    assert_eq!(
        summarize(r#"{"error": "no such collection"}"#),
        "no such collection"
    );
    assert_eq!(
        summarize(r#"{"message": "boom"}"#),
        r#"{"message": "boom"}"#
    );
    assert_eq!(summarize(" plain text \n"), "plain text");
}

#[test]
fn a_collection_that_is_not_open_has_no_vector_count() {
    let mut app = console();
    app.handle(press('2'));
    let ready = r#"{
        "ok": true, "version": "0.2.0", "data_dir": "/data", "total_collections": 2,
        "loaded_collections": 1, "total_vectors": 3,
        "collections": [
            {"name": "docs", "loaded": true, "integrity_ok": true},
            {"name": "cold", "loaded": false}
        ]
    }"#;
    app.handle(piramid::console::types::Event::Snapshot(Box::new(Ok(
        snapshot_from(&metrics_body(r#", "host": {}"#), ready),
    ))));
    let cold = app
        .collections
        .rows
        .iter()
        .find(|row| row.name == "cold")
        .expect("readiness lists cold");
    assert_eq!(cold.vectors(), None);
    let docs = app
        .collections
        .rows
        .iter()
        .find(|row| row.name == "docs")
        .expect("readiness lists docs");
    assert_eq!(docs.vectors(), Some(3));

    let drawn = screen(&mut app);
    let cold_line = drawn
        .lines()
        .find(|line| line.contains(" cold"))
        .expect("the sidebar lists cold");
    assert!(cold_line.contains('-'), "{cold_line}");
    assert!(!cold_line.contains(" 0 "), "{cold_line}");
}

#[test]
fn only_an_unreachable_server_gets_the_start_a_server_hint() {
    use piramid::console::client::ClientError;

    let mut app = console();
    app.handle(press('2'));
    app.handle(piramid::console::types::Event::Snapshot(Box::new(Err(
        ClientError::Unauthorized {
            path: "/api/metrics".into(),
            reason: "the server rejected the key".into(),
        },
    ))));
    let drawn = screen_of(&mut app, 200, 30);
    assert!(!drawn.contains("no server at"), "{drawn}");
    assert!(!drawn.contains("Start one with"), "{drawn}");
    assert!(drawn.contains("refused"), "{drawn}");

    let mut app = console();
    app.handle(press('2'));
    app.handle(piramid::console::types::Event::Snapshot(Box::new(Err(
        ClientError::Unreachable("/api/metrics".into(), "Connection refused".into()),
    ))));
    let drawn = screen_of(&mut app, 200, 30);
    assert!(drawn.contains("no server at"), "{drawn}");
}

#[test]
fn the_serve_unit_opens_the_address_the_configuration_binds() {
    let root = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("piramid-console-{}", std::process::id()));
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let mut config = piramid_core::config::Config::default();
    config.startup.bind = "0.0.0.0:7000".to_owned();
    config.console.base_url = "https://piramid.internal:6333".to_owned();
    let app = piramid::console::app::App::new(
        Settings::from_config(&config).unwrap(),
        Profile::Developer,
        root,
        &tx,
    )
    .expect("the log directory is creatable");
    let url = |id: &str| {
        app.units
            .iter()
            .find(|state| state.unit.id == id)
            .and_then(|state| state.unit.url.clone())
    };
    assert_eq!(url("serve").as_deref(), Some("http://localhost:7000"));
    assert_eq!(url("piramid").as_deref(), Some("http://localhost:6333"));
}

#[test]
fn settings_refuse_a_console_section_that_fails_validation() {
    let mut config = piramid_core::config::Config::default();
    config.console.log_lines = 0;
    assert!(Settings::from_config(&config).is_err());
}

#[test]
fn the_collection_detail_shows_documents_dimension_and_no_index() {
    let mut app = console_with_a_collection();
    assert_eq!(
        app.collections.current().and_then(|row| row.dimension()),
        Some(384)
    );
    let drawn = screen_of(&mut app, 200, 30);
    assert!(drawn.contains(" docs 3 documents "), "{drawn}");
    assert!(drawn.contains("dimension"), "{drawn}");
    assert!(drawn.contains("384"), "{drawn}");
    for stale in ["index", "rebuild", "hnsw", "ivf", "nprobe"] {
        assert!(!drawn.to_lowercase().contains(stale), "{stale} in {drawn}");
    }
}

#[test]
fn an_open_collection_with_no_vector_has_no_dimension() {
    let mut app = console();
    app.handle(press('2'));
    let list = r#"{"collections": [{"name": "docs", "count": 0, "created_at": 1,
        "updated_at": 1, "dimensions": null, "metric": "cosine"}]}"#;
    app.handle(piramid::console::types::Event::Snapshot(Box::new(Ok(
        snapshot_with(&metrics_body(r#", "host": {}"#), READY_BODY, list),
    ))));
    assert_eq!(
        app.collections.current().and_then(|row| row.dimension()),
        None
    );
    let drawn = screen_of(&mut app, 200, 30);
    assert!(drawn.contains("none, no vector stored yet"), "{drawn}");
}

#[test]
fn a_collection_list_without_the_dimensions_key_is_a_decode_error() {
    use piramid::console::client::{parse, ClientError, CollectionList};

    let error =
        parse::<CollectionList>("/api/collections", r#"{"collections": [{"name": "docs"}]}"#)
            .expect_err("dimensions is required");
    assert!(matches!(error, ClientError::Decode(..)), "{error:?}");
    assert!(error.to_string().contains("dimensions"), "{error}");
}

#[test]
fn the_collection_detail_shows_the_metric_beside_the_dimension() {
    let mut app = console_with_a_collection();
    assert_eq!(
        app.collections.current().and_then(|row| row.metric()),
        Some("euclidean")
    );
    let drawn = screen_of(&mut app, 200, 30);
    let at = |text: &str| drawn.find(text).expect("the detail pane draws the field");
    let (dimension, metric, memory) = (
        at("dimension         384"),
        at("metric            euclidean"),
        at("memory"),
    );
    assert!(dimension < metric && metric < memory, "{drawn}");
}

#[test]
fn a_collection_list_without_the_metric_key_is_a_decode_error() {
    use piramid::console::client::{parse, ClientError, CollectionList};

    let error = parse::<CollectionList>(
        "/api/collections",
        r#"{"collections": [{"name": "docs", "dimensions": 3}]}"#,
    )
    .expect_err("metric is required");
    assert!(matches!(error, ClientError::Decode(..)), "{error:?}");
    assert!(error.to_string().contains("metric"), "{error}");
}

#[test]
fn the_catalog_offers_the_model_recipes_and_says_what_serve_builds() {
    let units = catalog("http://localhost:6333");
    let hint = |id: &str| {
        units
            .iter()
            .find(|unit| unit.id == id)
            .map(|unit| unit.hint.clone())
            .unwrap_or_else(|| panic!("the catalog has {id}"))
    };
    assert!(hint("serve").contains("no model backend"));
    assert!(hint("piramid").contains("no model backend"));
    assert!(hint("bench-rag").contains("PIRAMID_BENCH_MODEL"));
    assert!(hint("test-model").contains("PIRAMID_TEST_MODEL"));
    assert!(hint("test-model-gpu").contains("CUDA"));
    assert!(hint("test-gpu").contains("CUDA"));
    for unit in &units {
        let hint = unit.hint.to_lowercase();
        for stale in ["index", "rebuild", "tuning", "vector database"] {
            assert!(!hint.contains(stale), "{}: {}", unit.id, unit.hint);
        }
    }
}

#[test]
fn every_key_hint_on_the_bottom_line_is_a_key_the_help_lists() {
    let mut app = console();
    for (digit, keys) in [
        ('2', &["j/k", "c", "R"][..]),
        ('3', &["j/k", "g", "R"][..]),
        ('4', &["h", "n", "R"][..]),
    ] {
        app.handle(press(digit));
        let bottom = screen_of(&mut app, 200, 30);
        app.handle(press('?'));
        let help = screen_of(&mut app, 200, 30);
        app.handle(press('x'));
        for key in keys {
            assert!(
                bottom.contains(&format!(" {key} ")),
                "{key} not hinted in view {digit}"
            );
            let spaced = key.replace('/', " / ");
            assert!(
                help.contains(&format!("  {spaced} ")),
                "{key} hinted in view {digit} but missing from its help: {help}"
            );
        }
    }
    app.handle(press('3'));
    let drawn = screen_of(&mut app, 200, 30);
    assert!(drawn.contains(" R re-read"), "{drawn}");
}
