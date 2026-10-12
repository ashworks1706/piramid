# Skills

| Skill | Purpose | Origin |
|---|---|---|
| `check` | The gate: fmt, clippy, tests, dependency rule, website lint | ours |
| `kernel-authoring` | Adding a compute backend or GPU kernel; layout and transfer rules | ours |
| `adr` | Write a decision record in `docs/decisions/` | ours |
| `roadmap` | Progress against `docs/ROADMAP.md` | ours |
| `code-quality` | Refactor pass: naming, reuse, registries, dead code. User-invoked. | ours (adapted) |
| `comment-style` | What a comment may say and which characters it may use | ours |
| `rust-skills` | 265 Rust rules across 26 categories — the style reference | [leonardomso/rust-skills](https://github.com/leonardomso/rust-skills), MIT |
| `test-driven-development` | red → green → refactor; test-quality anti-patterns | [obra/superpowers](https://github.com/obra/superpowers), MIT |
| `systematic-debugging` | Root cause before fixes | [obra/superpowers](https://github.com/obra/superpowers), MIT |
| `verification-before-completion` | Run the command, read the output, then claim done | [obra/superpowers](https://github.com/obra/superpowers), MIT |
| `security-audit-standard` | Secret scanning, input validation, authz, deps, OWASP | [0xMassi/claude-skills](https://github.com/0xMassi/claude-skills), MIT |
| `performance-audit-standard` | Hot-path identification, Big O, caching strategy | [0xMassi/claude-skills](https://github.com/0xMassi/claude-skills), MIT |
| `new-experiment` | Start an experiment in `apps/research` | [louped](https://github.com/ashworks1706/loupe) 0.8.0 templates, Apache-2.0 |
| `read-results` | Read a louped run and report it | louped templates |
| `write-report` | Write a run's report or an experiment's Result | louped templates |
| `ground-claims` | Trace every number and citation to a run or `sources/` | louped templates |
| `run-elsewhere` | Run an experiment on a cluster, VM or Colab | louped templates |
| `change-ui` | Change what louped's pages show | louped templates |
| `add-plugin` | Add a page, routes or tools from `apps/research/plugins/` | louped templates |

Third-party skills are vendored with their LICENSE files. Update by re-copying from the source;
don't edit them in place — local edits are silently lost on the next update, and the origin column
stops being true.

## Agents

| Agent | Use |
|---|---|
| `rust-reviewer` | Review a diff against AGENTS.md rules and architecture invariants |
| `kernel-reviewer` | Review compute backends, GPU kernels, and vector memory layout |

## When to use which

Writing Rust → `rust-skills`. Touching kernels or layout → `kernel-authoring`, then
`kernel-reviewer`. A bug → `systematic-debugging`. A feature or fix → `test-driven-development`.
Before claiming done → `verification-before-completion`, then `/check`. Before a release →
`security-audit-standard`. Optimizing → `performance-audit-standard`. A boundary change → `/adr`.
A research question → `new-experiment`; paths in the louped skills are relative to `apps/research`.
