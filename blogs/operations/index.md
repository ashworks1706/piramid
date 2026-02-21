# Operations

In this section of blogs, I will explain more about how vector databases are operated and maintained in production, the challenges involved, and how Piramid addresses those challenges. I will cover topics such as monitoring, alerting, scaling, backup and recovery, security, and best practices for operating vector databases in production.

## What's covered

<PostCards>
  <PostCard href="/blogs/operations/health" title="Health & Metrics">
    The health and readiness endpoints, what each one checks, and the key Prometheus metrics to watch in production.
  </PostCard>
  <PostCard href="/blogs/operations/logging" title="Logging & Tracing">
    How the tracing stack works, how to tune log verbosity, and what gets emitted per-request versus at startup.
  </PostCard>
  <PostCard href="/blogs/operations/maintenance" title="Maintenance">
    Triggering index rebuilds, running compaction, and using the duplicate-detection API to audit collection quality.
  </PostCard>
</PostCards>