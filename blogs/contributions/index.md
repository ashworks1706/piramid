# Contributions

If you want to contribute, thank you first — seriously. Piramid is still evolving fast, so the best contributions are the ones that are clear, scoped, and aligned with current roadmap work.

Before writing code, read the [roadmap](/blogs/contributions/roadmap) and pick something from there (or closely related to it). If your change is bigger than a small fix, open an issue first so we can align on approach before you spend time implementing.

## How to contribute

Use the usual fork + PR flow, but please keep PRs high-signal:

- Clear title and clear description.
- Explain what changed and why it changed.
- Link related issue(s).
- Include tests you ran.
- Add logs/screenshots when behavior is user-visible.

I care a lot about quality of writing in PRs and docs. Please include citations/sources when you’re making technical claims or using external references. Also, no AI slop: low-effort, generic generated text is not acceptable.

## Development expectations

At minimum, run:

- `cargo fmt`
- `cargo clippy --all-targets --all-features`
- `cargo test --locked`

For storage/search/index behavior changes, add or extend tests in `tests/` (or the nearest module test area). Prefer small, focused changes over broad rewrites.

Code style-wise: prefer `tracing` over `println!`, keep names explicit, and add comments only where intent is non-obvious. Avoid `unsafe` unless there is a measured need and a clear justification.

If you change API behavior, update docs accordingly and call out breaking changes directly in the PR description.

Also if you think the changes you made deserve to be in the blogs, please feel free to write a post about it! I’d love to share the spotlight and give credit to contributors who are doing great work.

## Scope notes

Current focus is search/index quality and performance. Zipy/GPU co-location is tracked separately and is not default in this repo yet. SDK and dashboard changes are welcome only when discussed first.

## Security / reporting

For security concerns, please contact the maintainer directly via GitHub profile instead of opening a public vulnerability issue.
