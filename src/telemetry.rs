use crate::APP_NAME;
use crate::state::AppState;

use axum::{
  extract::State,
  http::Request,
  middleware::Next,
  response::Response,
};
use const_format::formatcp;
use opentelemetry::sdk::{
    export::metrics::aggregation::cumulative_temporality_selector,
    metrics::{controllers::BasicController, selectors},
    trace::{self, RandomIdGenerator, Sampler, Tracer},
    Resource,
};
use opentelemetry::{Context, metrics, runtime, trace::TraceError, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use std::time::Duration;

const OTLP_GRPC_COLLECTOR_BASE_ENDPOINT: &str = "http://127.0.0.1:4317";
const OTLP_GRPC_COLLECTOR_TRACES_ENDPOINT: &str =
    formatcp!("{}/v1/traces", OTLP_GRPC_COLLECTOR_BASE_ENDPOINT);
const OTLP_GRPC_COLLECTOR_METRICS_ENDPOINT: &str =
    formatcp!("{}/v1/metrics", OTLP_GRPC_COLLECTOR_BASE_ENDPOINT);

// Logs are not yet supported.
//const OTLP_GRPC_COLLECTOR_LOGS_ENDPOINT: &str = formatcp!("{}/v1/logs", OTLP_GRPC_COLLECTOR_BASE_ENDPOINT);

pub fn init_tracer() -> Result<Tracer, TraceError> {
    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(OTLP_GRPC_COLLECTOR_TRACES_ENDPOINT)
                .with_timeout(Duration::from_secs(3)),
        )
        .with_trace_config(
            trace::config()
                .with_sampler(Sampler::AlwaysOn)
                .with_id_generator(RandomIdGenerator::default())
                .with_max_events_per_span(64)
                .with_max_attributes_per_span(16)
                .with_max_events_per_span(16)
                .with_resource(Resource::new(vec![KeyValue::new("service.name", APP_NAME)])),
        )
        .install_batch(opentelemetry::runtime::Tokio)
}

pub fn init_metrics() -> metrics::Result<BasicController> {
    opentelemetry_otlp::new_pipeline()
        .metrics(
            selectors::simple::inexpensive(),
            cumulative_temporality_selector(),
            runtime::Tokio,
        )
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(OTLP_GRPC_COLLECTOR_METRICS_ENDPOINT),
        )
        .with_period(Duration::from_secs(60))
        .with_timeout(Duration::from_secs(10))
        .build()
}

pub async fn metrics_middleware<B>(
    State(mut state): State<AppState>,
    request: Request<B>,
    next: Next<B>,
) -> Response {
  // do something with `request`...
  state.lock().request_counter.add(&Context::new(), 1, &[]);

  let response = next.run(request).await;
  // do something with `response`...

  response
}
