# Contributions

## What's covered

<PostCards>
  <PostCard href="/blogs/contributions/roadmap" title="Roadmap">
    Where the roadmap lives. Start there before picking up any work.
  </PostCard>
</PostCards>

If you want to contribute, thank you first -- seriously. Piramid is still evolving fast, so the best contributions are the ones that are clear, scoped, and aligned with current roadmap work.

Before writing code, read the [roadmap](https://github.com/ashworks1706/piramid/blob/main/docs/ROADMAP.md) and pick something from there (or closely related to it). If your change is bigger than a small fix, open an issue first so we can align on approach before you spend time implementing.

## How to contribute

Use the usual fork + PR flow, but please keep PRs high-signal:

- Clear title and clear description.
- Explain what changed and why it changed.
- Link related issue(s).
- Include tests you ran.
- Add logs/screenshots when behavior is user-visible.

I care a lot about quality of writing in PRs and docs. Please include citations/sources when you’re making technical claims or using external references. Also, no AI slop: low-effort, generic generated text is not acceptable.

## Development expectations

At minimum, run `just check` from the repository root. It runs the same gate as CI:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `scripts/check-deps.sh`, which enforces the dependency rule between crates
- the website lint

None of these needs a CUDA toolkit. `just check-features` compile-checks the `gpu-cuda` and `inference-candle` builds as well, and that one does.

For changes to storage, search or generation behavior, add or extend tests in the crate's `tests/` directory. Prefer small, focused changes over broad rewrites.

Code style-wise: use `tracing` rather than `println!`, keep names explicit, give every public item a `///` comment saying what it is, and follow the rules in [AGENTS.md](https://github.com/ashworks1706/piramid/blob/main/AGENTS.md). `unsafe` is denied outside five audited sites, and a new one needs a strong case.

If you change API behavior, update docs accordingly and call out breaking changes directly in the PR description.

Also if you think the changes you made deserve to be in the blogs, please feel free to write a post about it! I’d love to share the spotlight and give credit to contributors who are doing great work.

## Scope notes

Current focus is the inference engine: generation on one GPU, with retrieval from the document collections running in the same process. Approximate nearest neighbour indexes, clustering, and other vector database features are out of scope. SDK changes are welcome only when discussed first.

## Security / reporting

Report vulnerabilities through [GitHub Security Advisories](https://github.com/ashworks1706/piramid/security/advisories/new) instead of opening a public issue. [SECURITY.md](https://github.com/ashworks1706/piramid/blob/main/SECURITY.md) has the details.
