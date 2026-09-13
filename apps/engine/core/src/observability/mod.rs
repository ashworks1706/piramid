//! Telemetry export: tracing subscriber, optional OTLP spans, and Prometheus metrics.

pub mod prometheus;

use std::sync::OnceLock;

use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

pub use crate::config::{LogLevel, LoggingConfig, OtlpConfig, TelemetryConfig};

/// Holds exporters alive; dropping this flushes pending telemetry.
#[must_use = "dropping the guard shuts down telemetry export"]
pub struct ObservabilityGuard {
    // Shutting the provider down on drop flushes the last batch.
    #[cfg(feature = "otel")]
    otel: Option<opentelemetry_sdk::trace::SdkTracerProvider>,
}

impl Drop for ObservabilityGuard {
    fn drop(&mut self) {
        #[cfg(feature = "otel")]
        if let Some(provider) = self.otel.take() {
            if let Err(error) = provider.shutdown() {
                tracing::error!(
                    target: "piramid::observability",
                    %error,
                    "OTLP exporter failed to flush on shutdown"
                );
            }
        }
    }
}

/// Installs telemetry from configuration. Call once, early in main.
///
/// Returns None when logging is disabled or a subscriber is already installed, and an error when
/// a configured exporter cannot start.
pub fn install(
    logging: LoggingConfig,
    telemetry: &TelemetryConfig,
) -> crate::error::Result<Option<ObservabilityGuard>> {
    static INSTALLED: OnceLock<()> = OnceLock::new();
    if INSTALLED.set(()).is_err() {
        return Ok(None);
    }
    if !logging.enabled {
        return Ok(None);
    }
    init(telemetry, filter_for(logging)?, logging.json).map(Some)
}

/// Turn a [LoggingConfig] into a filter.
fn filter_for(logging: LoggingConfig) -> crate::error::Result<EnvFilter> {
    let directives = directives(level_directive(logging.level), logging);
    EnvFilter::try_new(&directives).map_err(|error| {
        crate::error::PiramidError::other(format!(
            "log filter '{directives}' does not parse: {error}"
        ))
    })
}

/// Build the filter string: a base level, then one off directive per subsystem switched off.
fn directives(base: &str, logging: LoggingConfig) -> String {
    let mut out = vec![base.to_string()];
    for (enabled, target) in [
        (logging.config, "piramid::config"),
        (logging.indexing, "piramid::indexing"),
        (logging.search, "piramid::search"),
        (logging.writes, "piramid::writes"),
        (logging.inference, "piramid::inference"),
        (logging.http, "piramid::http"),
    ] {
        if !enabled {
            out.push(format!("{target}=off"));
        }
    }
    out.join(",")
}

/// Level name for the tracing filter syntax.
fn level_directive(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Error => "error",
        LogLevel::Warn => "warn",
        LogLevel::Info => "info",
        LogLevel::Debug => "debug",
        LogLevel::Trace => "trace",
    }
}

/// Installs the tracing subscriber and any configured exporters.
fn init(
    config: &TelemetryConfig,
    filter: EnvFilter,
    json: bool,
) -> crate::error::Result<ObservabilityGuard> {
    let span_events = if config.span_events {
        FmtSpan::CLOSE
    } else {
        FmtSpan::NONE
    };

    let console = if json {
        tracing_subscriber::fmt::layer()
            .json()
            .with_target(true)
            .with_span_events(span_events)
            .boxed()
    } else {
        tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_span_events(span_events)
            .boxed()
    };

    let registry = tracing_subscriber::registry().with(filter).with(console);

    #[cfg(feature = "otel")]
    let otel_provider = match config.otlp.as_ref() {
        Some(otlp) => {
            let (layer, provider) = build_otel(otlp).map_err(|error| {
                crate::error::PiramidError::other(format!(
                    "OTLP exporter for {} failed to start: {error}",
                    otlp.endpoint
                ))
            })?;
            registry.with(layer).init();
            Some(provider)
        }
        None => {
            registry.init();
            None
        }
    };

    // Startup validation refuses an OTLP block on a build without the otel feature.
    #[cfg(not(feature = "otel"))]
    registry.init();

    tracing::info!(
        target: "piramid::observability",
        otlp = config.otlp.as_ref().map_or("off", |c| c.endpoint.as_str()),
        span_events = config.span_events,
        json_logs = json,
        "observability_ready"
    );

    Ok(ObservabilityGuard {
        #[cfg(feature = "otel")]
        otel: otel_provider,
    })
}

/// Builds the OTLP span-export layer and the provider that owns its background batcher.
#[cfg(feature = "otel")]
#[allow(clippy::type_complexity)]
fn build_otel<S>(
    cfg: &OtlpConfig,
) -> Result<
    (
        tracing_opentelemetry::OpenTelemetryLayer<S, opentelemetry_sdk::trace::Tracer>,
        opentelemetry_sdk::trace::SdkTracerProvider,
    ),
    Box<dyn std::error::Error>,
>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    use opentelemetry::trace::TracerProvider as _;
    use opentelemetry::KeyValue;
    use opentelemetry_otlp::WithExportConfig;
    use opentelemetry_sdk::Resource;

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(cfg.endpoint.clone())
        .build()?;

    let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(
            Resource::builder()
                .with_attributes([KeyValue::new("service.name", cfg.service_name.clone())])
                .build(),
        )
        .build();

    let tracer = provider.tracer("piramid");
    opentelemetry::global::set_tracer_provider(provider.clone());
    Ok((tracing_opentelemetry::layer().with_tracer(tracer), provider))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_produce_a_bare_level() {
        assert_eq!(directives("info", LoggingConfig::default()), "info");
    }

    #[test]
    fn a_disabled_subsystem_becomes_an_off_directive() {
        let logging = LoggingConfig {
            search: false,
            http: false,
            ..LoggingConfig::default()
        };
        assert_eq!(
            directives("info", logging),
            "info,piramid::search=off,piramid::http=off"
        );
    }

    #[test]
    fn the_filter_comes_from_the_configuration_only() {
        std::env::set_var("RUST_LOG", "trace");
        let logging = LoggingConfig {
            level: LogLevel::Warn,
            indexing: false,
            ..LoggingConfig::default()
        };
        let filter = filter_for(logging);
        std::env::remove_var("RUST_LOG");
        let filter = filter.unwrap().to_string();
        assert!(filter.contains("warn"), "{filter}");
        assert!(!filter.contains("trace"), "{filter}");
        assert!(filter.contains("piramid::indexing=off"), "{filter}");
    }

    #[test]
    fn every_level_maps_to_a_tracing_name() {
        for (level, name) in [
            (LogLevel::Error, "error"),
            (LogLevel::Warn, "warn"),
            (LogLevel::Info, "info"),
            (LogLevel::Debug, "debug"),
            (LogLevel::Trace, "trace"),
        ] {
            assert_eq!(level_directive(level), name);
        }
    }
}
