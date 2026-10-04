# 0014: Public source, proprietary licence

**Status:** accepted, 2026-10-04

**Decision.** The repository stays public and its licence is all rights reserved. The workspace is
not published to crates.io: every crate sets `publish = false`. Releases ship as binaries on this
repository's GitHub releases, which `piramiddb.com/install.sh` downloads, and as images on GHCR.
The Python client in `apps/sdk/python` is published to PyPI as `piramid` under MIT. Retrieval
strategies that come out of the research live in a private crate implementing `RetrievalHook` and
are linked into release builds; this repository carries the engine and the seam.

**Why.** The engine is useful to read and to build on, and a public tree earns issues, contributors
and credibility for the research. The edge Piramid is after is the method for retrieval inside the
forward pass, and that lives in the private crate. A proprietary licence keeps the engine from being
redistributed or resold while its source stays readable.

**What it forecloses.** `cargo install piramid` stops at 0.2.0, which stays public under Apache-2.0
along with 0.1.0 and 0.1.1. Outside contributions need terms that grant the project rights to them,
since the licence grants none by default.
