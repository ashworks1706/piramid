//! The catalog: every unit the repo can run, in sidebar order.

use crate::console::types::{Group, Unit};

/// Everything the console knows how to run, with the serve unit at server_url.
pub fn catalog(server_url: &str) -> Vec<Unit> {
    let mut units = processes(server_url);
    units.extend(tasks());
    units
}

fn processes(server_url: &str) -> Vec<Unit> {
    vec![
        process(
            "serve",
            &["serve"],
            "piramid serve, default build: HTTP API, no model backend",
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
            "compile-check gpu-cuda, inference-candle, all features",
        ),
        task_static(&["doc"], "rustdoc, warnings are errors"),
        task_static(&["bench"], "criterion, results in target/criterion"),
        task_static(
            &["bench-rag"],
            "end-to-end RAG benchmark; PIRAMID_BENCH_MODEL, _DATASET, _EMBEDDING",
        ),
        task_static(
            &["test-model"],
            "generation on the checkpoint in PIRAMID_TEST_MODEL",
        ),
        task_static(
            &["test-model-gpu"],
            "test-model with gpu-cuda, on the local CUDA device",
        ),
        task_static(&["test-gpu"], "device tests on the local CUDA device"),
        task_static(&["audit"], "advisories, bans, licences, sources"),
        named(
            "support-bundle",
            &["piramid", "support-bundle"],
            "diagnostics for a serve bug report",
        ),
        task_static(&["web-build"], "production build of the website"),
        task_static(&["web-shots"], "screenshots into target/screenshots"),
    ]
}

/// A one-shot recipe. The id is the recipe line itself, arguments included.
pub fn task(args: &[String], hint: &str) -> Unit {
    Unit {
        id: args.join(" "),
        group: Group::Tasks,
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
        args: args.iter().map(|arg| (*arg).into()).collect(),
        hint: hint.into(),
        url: url.map(Into::into),
    }
}
