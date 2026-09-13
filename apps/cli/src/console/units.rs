//! The catalog: every unit the repo can run, in sidebar order.
//!
//! One entry per just recipe or compose service driven from here. Each entry shells out to its
//! recipe.

use crate::console::types::{Group, Kind, Unit};

/// Everything the console knows how to run, with the serve unit at server_url.
pub fn catalog(server_url: &str) -> Vec<Unit> {
    let mut units = processes(server_url);
    units.extend(services());
    units.extend(tasks());
    units.extend(deploys());
    units
}

fn processes(server_url: &str) -> Vec<Unit> {
    vec![
        process(
            "serve",
            &["serve"],
            "the engine and its HTTP surface",
            Some(server_url),
        ),
        process(
            "web",
            &["web"],
            "dev server, hot reload; :web-preview for the real build",
            Some("http://localhost:3000"),
        ),
    ]
}

fn services() -> Vec<Unit> {
    vec![
        service(
            "piramid",
            None,
            "the server in a container, built from source",
            Some("http://localhost:6333"),
        ),
        service(
            "ollama",
            Some("ollama"),
            "local embeddings, so /embed works with no API key",
            Some("http://localhost:11434"),
        ),
    ]
}

fn tasks() -> Vec<Unit> {
    vec![
        task_static(&["doctor"], "required tools, .env, hooks"),
        task_static(&["bootstrap"], "everything a fresh clone needs"),
        task_static(&["setup"], "fetch every unit's dependencies"),
        task_static(
            &["check"],
            "the gate: fmt, clippy, tests, layering, website",
        ),
        task_static(&["check-rust"], "fmt, clippy, tests, dependency direction"),
        task_static(&["check-website"], "eslint over the website"),
        task_static(&["fmt"], "format every unit in place"),
        task_static(
            &["check-features"],
            "gpu-cuda, inference-candle, all-features",
        ),
        task_static(&["doc"], "rustdoc, warnings are errors"),
        task_static(&["bench"], "criterion, results in target/criterion"),
        task_static(&["audit"], "advisories, bans, licences, sources"),
        named(
            "support-bundle",
            &["piramid", "support-bundle"],
            "diagnostics for a bug report",
        ),
        task_static(&["web-build"], "production build of the website"),
        task_static(&["web-shots"], "screenshots into target/screenshots"),
    ]
}

fn deploys() -> Vec<Unit> {
    vec![
        deploy(&["up"], "dev stack: build locally and start", false),
        deploy(&["down"], "stop the stack", false),
        deploy(&["logs"], "follow every container", true),
        deploy(&["images"], "build the piramid image locally", false),
        deploy(
            &["prod-up"],
            "pull GHCR images (PIRAMID_IMAGE_TAG) and start",
            false,
        ),
        deploy(&["prod-down"], "stop the prod stack", false),
    ]
}

/// A one-shot recipe. The id is the recipe line itself, arguments included.
pub fn task(args: &[String], hint: &str) -> Unit {
    Unit {
        id: args.join(" "),
        group: Group::Tasks,
        kind: Kind::Task,
        args: args.to_vec(),
        hint: hint.into(),
        url: None,
    }
}

fn task_static(args: &[&str], hint: &str) -> Unit {
    let args: Vec<String> = args.iter().map(|arg| (*arg).into()).collect();
    task(&args, hint)
}

/// A task whose name is not its command line.
///
/// The sidebar and the start command take the name; args carries the recipe line.
fn named(id: &str, args: &[&str], hint: &str) -> Unit {
    Unit {
        id: id.into(),
        ..task_static(args, hint)
    }
}

fn process(id: &str, args: &[&str], hint: &str, url: Option<&str>) -> Unit {
    Unit {
        id: id.into(),
        group: Group::Apps,
        kind: Kind::Process,
        args: args.iter().map(|arg| (*arg).into()).collect(),
        hint: hint.into(),
        url: url.map(Into::into),
    }
}

fn service(id: &str, profile: Option<&str>, hint: &str, url: Option<&str>) -> Unit {
    Unit {
        id: id.into(),
        group: Group::Containers,
        kind: Kind::Service {
            service: id.into(),
            profile: profile.map(Into::into),
        },
        args: Vec::new(),
        hint: hint.into(),
        url: url.map(Into::into),
    }
}

/// A recipe in the deploy section. Setting follows marks the ones that stream until stopped.
fn deploy(args: &[&str], hint: &str, follows: bool) -> Unit {
    let mut unit = task_static(args, hint);
    unit.group = Group::Deploy;
    if follows {
        unit.kind = Kind::Process;
    }
    unit
}
