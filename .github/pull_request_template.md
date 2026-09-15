## What and why

<!-- One logical change. What it does, and why; the diff shows how. -->

## Checklist

- [ ] `just check` passes (the pre-commit hook runs the same recipes)
- [ ] New behaviour has a test, and the test fails without the change
- [ ] No new edge in the dependency rule, or it was agreed in an issue first
- [ ] No contradiction with `docs/ARCHITECTURE.md` or `docs/ROADMAP.md`, or the doc is updated here
- [ ] A new tunable is in `config.example.yaml` at its default
- [ ] No `#[allow]` or skipped test added to get the gate green; a genuine exception is scoped to
      the smallest item with a one-line reason
- [ ] A public item has a doc comment saying what it is
