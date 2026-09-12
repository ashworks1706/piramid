# Security

## Reporting

Report vulnerabilities through
[GitHub Security Advisories](https://github.com/ashworks1706/piramid/security/advisories/new).
Please don't open a public issue for something exploitable.

Include what an attacker gains, how to reproduce it, and the affected version. Expect an
acknowledgement within a week.

## Supported versions

Pre-1.0, so only the latest release gets fixes.

## Threat model

Piramid has one API key and no authorization beyond it. A client holding the key can read, write,
and delete every collection.

- With `PIRAMID_API_KEY` set, every route except `/api/health` and `/api/readyz` requires
  `Authorization: Bearer <key>`, compared in constant time. The key is read from the environment
  only; a configuration file cannot hold it.
- With no key, the server serves only a loopback address. Binding anything else fails at startup
  unless `startup.http.auth.allow_unauthenticated` is set, and that switch together with a key is
  also an error.
- Each client IP gets a token bucket (`startup.http.rate_limit`, 100 requests per second with a
  burst of 200 by default). Behind a reverse proxy every client shares the proxy's bucket.

Run it on a trusted network or behind a gateway that terminates TLS.

Specifically:

- CORS is wide open (`allow_origin(Any)`), so any web page can call the API from a browser.
  Without a key, someone visiting a hostile page while a local Piramid is running can have their
  data read or destroyed. Set `PIRAMID_API_KEY` even on localhost if that matters, or restrict
  origins in your reverse proxy.
- There's no transport encryption. Terminate TLS upstream.
- Collections are not a security boundary.
- The body limit is 100 MB, so a caller within its rate limit can still exhaust memory or disk.
  `startup.disk.min_free_bytes` bounds the disk case only.
- `/api/readyz` is unauthenticated and names the data directory and every collection.

## Telemetry

Nothing is transmitted to this project under any configuration. The exporters in
`core::observability` points at endpoints you supply, and `PIRAMID_LOG_SPANS` only writes to your
own logs.

Span fields carry collection names and request ids. They never carry vector contents, document
text, or metadata values.

## Diagnostic bundles

`piramid support-bundle` writes version, platform, build features, resolved configuration, and
collection state to a file for attaching to a bug report. Variables whose names look like
credentials — containing `KEY`, `TOKEN`, `SECRET`, `PASSWORD`, `DSN`, `CREDENTIAL`, or `AUTH` —
are reported as `<redacted, N chars>`, since whether a key is set is diagnostic but its value
never is.

The bundle still contains your configuration and collection names. Read it before sharing.

## Handling secrets

`PIRAMID_API_KEY`, `OPENAI_API_KEY` and other provider credentials come from the environment and
belong in `.env`, which is gitignored. They're never logged. Don't put them in a compose file or an image.

## Dependencies

`cargo deny check advisories bans licenses sources` runs in CI weekly and on every PR, or locally
as `just audit`. Dependabot proposes updates weekly.

## Unsafe code

`unsafe_code` is denied workspace-wide, and permitted at four audited sites: `as_bytes` and
`as_bytes_mut` in `apps/engine/hardware/src/gpu/buffer.rs`, which reinterpret a typed slice as
bytes for device transfer; `database::storage::sidecars::mmap::create_mmap`; and
`serving::disk`. Each carries a `// SAFETY:` comment stating its precondition. A PR introducing
`unsafe` anywhere else fails CI.
