# Maintenance

TODO cover:
- Rebuild index: endpoint, background job status, when to trigger, expected impact.
- Compaction: what it reclaims, how to run, metrics to verify.
- Duplicate detection: API, threshold/k/ef/nprobe knobs, use cases.
- Limits/guards: how writes behave when max vectors/bytes or disk guard triggers.
- Backup/recovery guidance: checkpoints + WAL replay, safe snapshot approach.
