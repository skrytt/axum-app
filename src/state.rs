use crate::APP_NAME;

use opentelemetry::{global, metrics::Counter};
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Clone)]
pub struct AppState {
    // TODO: find a more performant solution than Mutexes for
    // thread-safe incrementing of metrics.
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
    pub request_counter: Counter<u64>,
    pub response_counter: Counter<u64>,
}

impl AppStateInner {
    pub fn new() -> Self {
        let _tracer = global::tracer(APP_NAME);
        let meter = global::meter(APP_NAME);

        let request_counter = meter.u64_counter("requests").init();
        let response_counter = meter.u64_counter("responses").init();

        Self {
            request_counter,
            response_counter,
        }
    }
}
