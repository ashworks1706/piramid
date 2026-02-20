# Overview

## How to contribute

- **Focus areas right now:** search/index improvements and performance work; GPU co-location/Zipy kernel is out-of-scope for this repo today. SDKs and dashboard changes are also out-of-scope unless discussed first.
- **Workflow:** fork + PR. Use clear PR titles. Include:
  - What changed (1–2 sentences)
  - Root cause and fix summary
  - Screenshots/log snippets for user-visible or routing changes
  - Tests you ran
- **Issues:** please open an issue before significant work so we can align on approach. Start by scanning `blogs/roadmap/index.md` and open issues tied to those items or adjacent bugs you find.

## Development expectations

- **Testing:** run at least:
  - `cargo fmt`
  - `cargo clippy --all-targets --all-features` (or `--locked` if you prefer)
  - `cargo test --locked`
  - For behavioral changes in storage/search, add/extend tests in `tests/` or the relevant module’s tests file.
- **Style:**
  - Prefer `tracing` over `println!`; keep logs structured and concise.
  - Keep variable and function names clear; avoid acronyms unless obvious.
  - Add brief comments only where the intent isn’t obvious (index/search internals, persistence).
  - Split large modules into focused submodules (e.g., storage/persistence, index/…).
  - Avoid `unsafe` unless there’s a measurable need and it’s well-justified.
- **API/behavior changes:** update README and blogs/roadmap when applicable. Call out breaking changes in the PR description.

## Security / reporting

For security concerns, please email the maintainer (GitHub profile). Avoid filing public issues for potential vulnerabilities.
