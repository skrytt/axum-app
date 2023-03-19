/// Toying about with Axum, Tracing, OpenTelemetry.
/// This is for my learning purposes, don't take it as best practice but by all means
/// try it out if you like!
///
/// This code is designed to forward telemetry to the OpenTelemetry collector over gRPC.
/// You can then configure the OpenTelemetry collector to send those onwards wherever
/// you need (the OpenTelemetry configuration is out of scope of what's in this repo).

mod state;
mod telemetry;

use state::AppState;
use telemetry::{init_metrics, init_tracer, metrics_middleware};

use axum::{
    body::Bytes,
    http::{HeaderMap, Request},
    middleware,
    response::{Html, Response},
    routing::get,
    Router,
};
use std::{
  error::Error,
  net::SocketAddr,
  time::Duration
};
use tower_http::{
  classify::ServerErrorsFailureClass,
  trace::TraceLayer
};
use tracing::Span;

// TODO: pass metadata such as APP_NAME via the environment.
const APP_NAME: &str = "axum-app";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
  let _ = init_tracer();
  init_metrics()?;

  // TODO: instrument Metrics via a stateful middleware function.

  let state = AppState::new();

  let app = Router::new()
    .route("/", get(handler))

    .with_state(AppState::new())

    // TODO: find a clean way to lift this layer into `src/telemetry.rs`.
    // last I tried, struggled a bit with generic function signatures and closures.
    .layer(
      TraceLayer::new_for_http()
        .on_request(|request: &Request<_>, _span: &Span| {
            tracing::debug!("Receved {} request", request.method());
            tracing::debug!("{:?}", request);
        })
        .on_response(|response: &Response, latency: Duration, _span: &Span| {
            tracing::debug!(
                "Sending {} response (took {} us)",
                response.status(),
                latency.as_micros()
            );
            tracing::debug!("{:?}", response);
        })
        .on_body_chunk(|chunk: &Bytes, latency: Duration, _span: &Span| {
            tracing::debug!(
                "Response body produced a new chunk ({} bytes, took {} us)",
                chunk.len(),
                latency.as_micros()
            );
            tracing::debug!("{:?}", chunk);
        })
        .on_eos(
            |_trailers: Option<&HeaderMap>, stream_duration: Duration, _span: &Span| {
                tracing::debug!(
                    "Streaming response body ended (took {} us)",
                    stream_duration.as_micros()
                );
            },
        )
        .on_failure(
            |error: ServerErrorsFailureClass, latency: Duration, _span: &Span| {
                tracing::debug!("Error: {} (took {} us)", error, latency.as_micros());
            },
        ))
    .layer(middleware::from_fn_with_state(state.clone(), metrics_middleware));

  let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
  tracing::debug!("listening on {}", addr);
  axum::Server::bind(&addr)
      .serve(app.into_make_service())
      .await?;

  Ok(())
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}
