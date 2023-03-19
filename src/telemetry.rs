use crate::APP_NAME;
use crate::state::AppState;

use axum::{
  extract::State,
  http::Request,
  middleware::Next,
  response::Response,
};
use opentelemetry::sdk::{
    export::metrics::aggregation::cumulative_temporality_selector,
    metrics::{controllers::BasicController, selectors},
    trace::{self, RandomIdGenerator, Sampler},
    Resource,
};
use opentelemetry::{Context, metrics, runtime, trace::TraceError, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

const OTLP_GRPC_COLLECTOR_ENDPOINT: &str = "http://127.0.0.1:4317/";

pub fn init_tracer() -> Result<(), TraceError> {
    println!("Will push traces to: {}", OTLP_GRPC_COLLECTOR_ENDPOINT);
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(OTLP_GRPC_COLLECTOR_ENDPOINT)
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
        .install_batch(opentelemetry::runtime::Tokio)?;

  tracing_subscriber::registry()
    .with(
      tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "example_tracing_aka_logging=debug,tower_http=debug".into()),
    )
    .with(tracing_opentelemetry::layer().with_tracer(tracer))
    .init();

  Ok(())
}

pub fn init_metrics() -> metrics::Result<BasicController> {
    println!("Will push metrics to: {}", OTLP_GRPC_COLLECTOR_ENDPOINT);
    opentelemetry_otlp::new_pipeline()
        .metrics(
            selectors::simple::inexpensive(),
            cumulative_temporality_selector(),
            runtime::Tokio,
        )
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(OTLP_GRPC_COLLECTOR_ENDPOINT),
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
  // Opentelemetry Context objects allow propagation of metrics across
  // execution boundaries. We don't need that, we just create a
  // new context to satisfy the metric update APIs.
  let context = Context::new();

  // Request metrics
  // TODO don't create a new context for every request
  state.lock().request_counter.add(
    &context,
    1,
    &[
      KeyValue::new("service.name", APP_NAME),
    ]
  );

  // Run the inner middlewares
  let response = next.run(request).await;

  // Response metrics
  state.lock().response_counter.add(
    &context,
    1,
    &[
        KeyValue::new("service.name", APP_NAME),
        KeyValue::new("http.response.status", response.status().as_u16().to_string())
    ]
  );

  response
}
