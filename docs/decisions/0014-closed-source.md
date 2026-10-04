# 0014: Closed source

**Status:** accepted, 2026-10-04

**Decision.** The repository is private and the workspace is not published. Every crate sets
`publish = false`, there is no crates.io release pipeline, and the licence is all rights reserved.
New releases ship as binaries and images built from tags. The website states what Piramid is and
publishes results; it reads nothing from the repository at build time.

**Why.** The speedups Piramid is after come from its own research on retrieval inside the forward
pass. A crates.io release uploads the full source, so publishing and keeping that research private
cannot both hold. A site that renders the roadmap and architecture from the tree leaks the same
material on every build.

**What it forecloses.** `cargo install piramid` stops at 0.2.0, which stays public under Apache-2.0
along with 0.1.0 and 0.1.1. Build provenance attestations and dependency review are gone, because
GitHub offers them on private repositories only on paid plans; `cargo deny` still covers
advisories, licences and sources.

**The public surface.** The public repository `ashworks1706/piramid-sdk` holds the MIT-licensed
clients and takes issues, and its releases carry the engine binaries, uploaded by `release.yml`.
`piramiddb.com/install.sh` downloads them, and the GHCR images stay public. The Python client is
published to PyPI as `piramid` from that repository.
