use crate::APP_NAME;

use opentelemetry::{
    global,
    metrics::{Counter, Meter},
};
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Clone)]
pub struct AppState {
    inner: Arc<Mutex<AppStateInner>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(AppStateInner::new())),
        }
    }

    pub fn lock(&mut self) -> MutexGuard<AppStateInner> {
        self.inner.lock().unwrap()
    }
}

pub struct AppStateInner {
    tracer: opentelemetry::global::BoxedTracer,
    meter: Meter,

    pub request_counter: Counter<u64>,
}

impl AppStateInner {
    pub fn new() -> Self {
        let tracer = global::tracer(APP_NAME);
        let meter = global::meter(APP_NAME);

        let request_counter = meter.u64_counter("requests").init();

        Self {
            tracer,
            meter,
            request_counter,
        }
    }
}
