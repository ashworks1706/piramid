# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0](https://github.com/ashworks1706/piramid/releases/tag/piramid-model-v0.3.0) - 2026-10-04

### Other

- Restructure the engine into nine crates by topic
- Fold fusion, observability, metadata and quantization into their owners
- Delete dead configuration and give a metric one spelling
- Rename the folders and files named for Rust constructs
- Fold retrieval and collections into database, and delete the architecture blogs
- Rewrite every comment to say what the code does
- Document every public item in core, model and database
- Merge branch 'worktree-agent-ad95bbdc3a25261e9' into engine-honesty
- Warn on missing docs across the workspace
- Run Qwen2 and Qwen3 on candle through a layer-by-layer driver
- Schedule, batch and stream generations through an engine thread
- Serve generations over /api/generate and OpenAI-compatible /v1
- Hand device hidden states to hooks and embed in-process
- Divide device memory into weights, cache and index pools
- Describe the inference engine where the docs said scaffolding
- Borrow sequence tokens for the hook instead of copying them per step
- Strip rationale and restatement out of comments across the tree
- Tighten Rust fundamentals across the inference engine
- Refuse instead of falling back, and report only what was measured
- Keep every test in tests/ and every config type in core
- Remove ANN index families and database-only features
- Refocus the docs and site on serving RAG from one GPU
- Make the CLI and docs serve-first, and keep comments to one line
- roadmap
- Prepare the repository to go private
- Publish the crates to crates.io again
