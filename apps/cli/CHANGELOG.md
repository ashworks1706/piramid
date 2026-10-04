# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0](https://github.com/ashworks1706/piramid/compare/piramid-v0.2.0...piramid-v0.3.0) - 2026-10-04

### Other

- added gpu refactor
- Update README.md
- Update README.md
- Update README.md
- Split into an 11-crate workspace with enforced layering
- Add AGENTS.md, architecture, roadmap, and seven ADRs
- Add observability: OTLP traces, Sentry errors, Prometheus metrics
- Rework CI/CD for the workspace; add per-crate READMEs
- Reorganize into engine/ and apps/; split fusion; add support-bundle
- Fold the retrieval hook back into inference as a module
- Audit names for ambiguity; strip narrating comments
- Rename core::telemetry to core::stats
- Remove the Sentry integration
- Rewrite docs in plainer prose; second pass on comments
- Separate contributor tooling from the shipped CLI in the docs
- Tighten comments; fix CI advisories, Docker toolchain, npm audit
- Strip restated and narrated comments across the workspace
- Document the tech stack
- roadmap update
- Delete the compute fallback and the compat aliases
- Give every write and query one request shape
- Deny unwrap and expect outside tests
- Cut the long doc comments and the dead code they described
- Managers name domains: cache crate, quantization move, SidecarManager
- Add EmbeddingsManager; managers import from the crate root
- Update README crate count and dependency graph for piramid-cache
- Rename compute/backends to strategies; ADR 0013
- Flatten server/runtime, drop http/helpers, document the layer split
- Delete VectorSlab and SlabVectorReader; keep the seam
- Fold piramid-cache back into collections; list the managers in the facade
- Modernize idioms; enforce them with lints
- Reduce duplication and nesting across the workspace
- Consolidate config and error placement; remove duplicate representations
- Split config by lifecycle and make the whole surface one file
- Reorder the roadmap around retrieval during generation
- Drop the advisory voice from the docs
- Flatten the engine tree and remove the decision log
- Restructure the engine into nine crates by topic
- Fold fusion, observability, metadata and quantization into their owners
- Move quantization back under compute and telemetry setup into core
- Close the placement gaps four audits found
- Fix /api/readyz reporting a healthy server as not ready
- Delete dead configuration and give a metric one spelling
- Move Document and Hit into core, and fold Hit's duplicated fields
- Fold retrieval and collections into database, and delete the architecture blogs
- Say what Piramid is: memory, not an inference engine for retrieval
- Flatten the positioning prose back to the repo's register
- Restore the inference-engine framing
- Add `piramid top`, a live view of a running server
- Open the developer console when piramid is run with no subcommand
- Name console tasks for what they are, not how they run
- Rewrite every comment to say what the code does
- Make the console the whole CLI, with a view per concern
- Say which server is unreachable instead of waiting on it
- Move console settings into the configuration file
- Update README.md
- Make serve safe to expose: auth, rate limit, graceful stop
- Merge branch 'worktree-agent-a206844084d316161' into engine-honesty
- Upgrade ratatui and stop the console hiding failures
- Merge branch 'worktree-agent-aed97a763626ae944' into engine-honesty
- Refuse compose rows without an exit code, and check the justfile test
- Report GPU memory, utilisation and temperature beside host readings
- Serve generations over /api/generate and OpenAI-compatible /v1
- Graph generation and show the KV cache in the device view
- Divide device memory into weights, cache and index pools
- Show the device memory budget in the console device view
- Strip rationale and restatement out of comments across the tree
- Tighten Rust fundamentals across the inference engine
- Refuse instead of falling back, and report only what was measured
- Keep every test in tests/ and every config type in core
- Remove ANN index families and database-only features
- Refocus the docs and site on serving RAG from one GPU
- Make the CLI and docs serve-first, and keep comments to one line
- Update README.md
- Update logo in README.md
- cli
- Update project description in README.md
- Modify description in README.md
- roadmap
- Prepare the repository to go private
- Draw the Piramid mark as a pixel-block pyramid everywhere
- Cut the README to essentials and give each doc one job
- Publish user docs at /docs and ship inference in releases
- Render the user docs with Fumadocs
- Keep the repository public with the SDK in the monorepo
- Fix the CUDA image build and document installing with cargo
- Publish the crates to crates.io again
- Remove Docker images, compose and the console's container units
