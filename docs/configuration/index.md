# Configuration (outline)

Piramid loads a single `AppConfig` at startup (env vars + file). Document defaults and how to override.

## Loading
- `piramid init --path piramid.yaml` to scaffold a config file.
- Env overrides: `APP__SECTION__FIELD` style (match your loader) or direct provider vars (e.g., `EMBEDDING_PROVIDER`).
- CLI flags: `--config`, `--data-dir`, etc. (fill in as you add them).

## Sections to cover
- `index`: metric, execution mode, search defaults (ef/nprobe/filter_overfetch).
- `quantization`: PQ/disk-only toggles (future).
- `memory`: mmap on/off, initial mmap size, cache caps.
- `wal`: enabled, checkpoint frequency/interval, max log size, sync on write.
- `parallelism`: thread/parallel search tuning.
- `limits`: max vectors/bytes/vector bytes per collection, disk read-only thresholds.
- `embedding`: provider, timeouts, retry/backoff.

## Validation
- Add a `validate()` pass at startup for required fields and safe limits.
- Log resolved config once on boot for observability.

> Keep this page updated as new knobs land; add a table of env vars and their defaults.
