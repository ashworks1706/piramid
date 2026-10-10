# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0](https://github.com/ashworks1706/piramid/releases/tag/piramid-core-v0.3.0) - 2026-10-10

### Other

- Flatten core; split data/ from retrieval/
- Audit names for ambiguity; strip narrating comments
- Rename core::telemetry to core::stats
- Remove the Sentry integration
- Rewrite docs in plainer prose; second pass on comments
- Tighten comments; fix CI advisories, Docker toolchain, npm audit
- Strip restated and narrated comments across the workspace
- Delete the compute fallback and the compat aliases
- Give every write and query one request shape
- Collapse the embedding providers and the remaining fallbacks
- Deny unwrap and expect outside tests
- Cut the long doc comments and the dead code they described
- Managers name domains: cache crate, quantization move, SidecarManager
- Rename compute/backends to strategies; ADR 0013
- Modernize idioms; enforce them with lints
- Reduce duplication and nesting across the workspace
- Consolidate config and error placement; remove duplicate representations
- Split config by lifecycle and make the whole surface one file
- Restructure the engine into nine crates by topic
- Fold fusion, observability, metadata and quantization into their owners
- Move quantization back under compute and telemetry setup into core
- Close the placement gaps four audits found
- Delete dead configuration and give a metric one spelling
- Move Document and Hit into core, and fold Hit's duplicated fields
- Rename the folders and files named for Rust constructs
- Scaffold the inference, fusion and cache configuration surface
- Rewrite every comment to say what the code does
- Move console settings into the configuration file
- Give every CPU strategy real batch kernels and drop silent substitutes
- Refuse searches and opens that would use the wrong metric
- Move to bincode 2 and yaml_serde, off unmaintained parsers
- Merge branch 'worktree-agent-a0100051d8f1693c5' into engine-honesty
- Refuse configuration nothing applies, and fail when telemetry cannot start
- Report host CPU and memory, and graph them in a device view
- Apply the embedding cache and options the configuration names
- Merge branch 'worktree-agent-a6722a1b028a91002' into engine-honesty
- Make serve safe to expose: auth, rate limit, graceful stop
- Merge branch 'worktree-agent-a206844084d316161' into engine-honesty
- Make a reload reach open collections, from the file the server booted
- Grow an auto index into the family its size picks
- Document every public item in core, model and database
- Merge branch 'worktree-agent-ad95bbdc3a25261e9' into engine-honesty
- Warn on missing docs across the workspace
- Validate every write before it is logged or stored
- Run Qwen2 and Qwen3 on candle through a layer-by-layer driver
- Schedule, batch and stream generations through an engine thread
- Serve generations over /api/generate and OpenAI-compatible /v1
- Hand device hidden states to hooks and embed in-process
- Divide device memory into weights, cache and index pools
- Strip rationale and restatement out of comments across the tree
- Tighten Rust fundamentals across the inference engine
- Refuse instead of falling back, and report only what was measured
- Keep every test in tests/ and every config type in core
- Remove ANN index families and database-only features
- Make the CLI and docs serve-first, and keep comments to one line
- cli
- Prepare the repository to go private
- Publish the crates to crates.io again
