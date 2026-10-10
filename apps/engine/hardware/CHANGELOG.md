# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0](https://github.com/ashworks1706/piramid/releases/tag/piramid-hardware-v0.3.0) - 2026-10-10

### Other

- Move engine/ and assets/ under apps/
- Flatten core; split data/ from retrieval/
- Rewrite docs in plainer prose; second pass on comments
- Audit the roadmap against the code and reorder it
- Tighten comments; fix CI advisories, Docker toolchain, npm audit
- Strip restated and narrated comments across the workspace
- Delete the compute fallback and the compat aliases
- Deny unwrap and expect outside tests
- Cut the long doc comments and the dead code they described
- Managers name domains: cache crate, quantization move, SidecarManager
- Rename compute/backends to strategies; ADR 0013
- Delete the last dead code; record the graceful-shutdown gap
- Make type names match their folders after the strategies rename
- Modernize idioms; enforce them with lints
- Reduce duplication and nesting across the workspace
- Consolidate config and error placement; remove duplicate representations
- Split config by lifecycle and make the whole surface one file
- Restore compute's thiserror dependency and fill in the bench
- Reorder the roadmap around retrieval during generation
- Drop the advisory voice from the docs
- Flatten the engine tree and remove the decision log
- Restructure the engine into nine crates by topic
- Fold fusion, observability, metadata and quantization into their owners
- Move quantization back under compute and telemetry setup into core
- Delete dead configuration and give a metric one spelling
- Rewrite every comment to say what the code does
- Give every CPU strategy real batch kernels and drop silent substitutes
- Refuse configuration nothing applies, and fail when telemetry cannot start
- Report host CPU and memory, and graph them in a device view
- Merge branch 'worktree-agent-a6722a1b028a91002' into engine-honesty
- Make a reload reach open collections, from the file the server booted
- Warn on missing docs across the workspace
- Wire the CUDA device runtime and device distance kernels
- Default CUDA_HOME for cargo and gate the GPU refusal test on the feature
- Report GPU memory, utilisation and temperature beside host readings
- Divide device memory into weights, cache and index pools
- Strip rationale and restatement out of comments across the tree
- Tighten Rust fundamentals across the inference engine
- Refuse instead of falling back, and report only what was measured
- Keep every test in tests/ and every config type in core
- Remove ANN index families and database-only features
- Make the CLI and docs serve-first, and keep comments to one line
- Prepare the repository to go private
- Reserve device memory with a compare-exchange loop
- Publish the crates to crates.io again
