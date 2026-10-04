# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0](https://github.com/ashworks1706/piramid/releases/tag/piramid-serving-v0.3.0) - 2026-10-04

### Other

- Restructure the engine into nine crates by topic
- Fold fusion, observability, metadata and quantization into their owners
- Close the placement gaps four audits found
- Fix /api/readyz reporting a healthy server as not ready
- Delete dead configuration and give a metric one spelling
- Give each knob and each sidecar one name
- Move Document and Hit into core, and fold Hit's duplicated fields
- Rename collections modules to say what they hold
- Rename the folders and files named for Rust constructs
- Rank every search by the metric the request asked for
- Fold retrieval and collections into database, and delete the architecture blogs
- Scaffold the inference, fusion and cache configuration surface
- Key WAL metrics by collection name, not by file path
- Store a collection's vectors as one contiguous slab
- Rewrite every comment to say what the code does
- Document every public item in piramid-serving
- Merge branch 'worktree-agent-a405b361759546f9a' into engine-honesty
- Validate collection names where they become paths, and lift read-only
- Refuse a second rebuild, page in id order, and stop skipping lost records
- Warn on missing docs across the workspace
- Validate every write before it is logged or stored
- Report GPU memory, utilisation and temperature beside host readings
- Serve generations over /api/generate and OpenAI-compatible /v1
- Divide device memory into weights, cache and index pools
- Merge branch 'rag-bench' into inference-engine
- Open the GPU the benchmark's device arm and cuda model need
- Strip rationale and restatement out of comments across the tree
- Tighten Rust fundamentals across the inference engine
- Refuse instead of falling back, and report only what was measured
- Keep every test in tests/ and every config type in core
- Remove ANN index families and database-only features
- Refocus the docs and site on serving RAG from one GPU
- Make the CLI and docs serve-first, and keep comments to one line
- Prepare the repository to go private
- Publish the crates to crates.io again
