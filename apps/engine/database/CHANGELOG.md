# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0](https://github.com/ashworks1706/piramid/releases/tag/piramid-database-v0.3.0) - 2026-10-04

### Other

- Restructure the engine into nine crates by topic
- Fold fusion, observability, metadata and quantization into their owners
- Delete dead configuration and give a metric one spelling
- Give each knob and each sidecar one name
- Move Document and Hit into core, and fold Hit's duplicated fields
- Split index/traits.rs and stop persistence naming its own parent
- Honour sync_on_write and max_log_size
- Fold retrieval and collections into database, and delete the architecture blogs
- Scaffold the inference, fusion and cache configuration surface
- Fix the intra-doc link rustdoc rejects
- Group the collection domain into its own folder
- Store a collection's vectors as one contiguous slab
- Rewrite every comment to say what the code does
- Document every public item in core, model and database
- Merge branch 'worktree-agent-ad95bbdc3a25261e9' into engine-honesty
- Warn on missing docs across the workspace
- Validate every write before it is logged or stored
- Strip rationale and restatement out of comments across the tree
- Refuse instead of falling back, and report only what was measured
- Keep every test in tests/ and every config type in core
- Remove ANN index families and database-only features
- Refocus the docs and site on serving RAG from one GPU
- Make the CLI and docs serve-first, and keep comments to one line
- Prepare the repository to go private
- Publish the crates to crates.io again
