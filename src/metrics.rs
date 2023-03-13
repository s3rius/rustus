use std::collections::HashMap;

use crate::errors::RustusResult;

#[allow(clippy::module_name_repetitions)]
#[derive(Clone)]
pub struct RustusMetrics {
    pub started_uploads: prometheus::IntCounter,
    pub finished_uploads: prometheus::IntCounter,
    pub active_uploads: prometheus::IntGauge,
    pub upload_sizes: prometheus::Histogram,
    pub terminated_uploads: prometheus::IntCounter,
    pub found_errors: prometheus::IntCounterVec,
    pub registry: prometheus::Registry,
}

impl RustusMetrics {
    pub fn new() -> RustusResult<Self> {
        let registry = prometheus::Registry::new();
        let started_uploads =
            prometheus::IntCounter::new("started_uploads", "Number of created uploads")?;
        let finished_uploads =
            prometheus::IntCounter::new("finished_uploads", "Number of finished uploads")?;
        let active_uploads =
            prometheus::IntGauge::new("active_uploads", "Number of active file uploads")?;
        let upload_sizes = prometheus::Histogram::with_opts(
            prometheus::HistogramOpts::new("uploads_sizes", "Size of uploaded files in bytes")
                .buckets(prometheus::exponential_buckets(2., 2., 40)?),
        )?;
        let terminated_uploads =
            prometheus::IntCounter::new("terminated_uploads", "Number of terminated uploads")?;
        let found_errors = prometheus::IntCounterVec::new(
            prometheus::Opts {
                namespace: String::new(),
                subsystem: String::new(),
                name: "errors".into(),
                help: "Found errors".into(),
                const_labels: HashMap::new(),
                variable_labels: Vec::new(),
            },
            &["path", "description"],
        )?;

        registry.register(Box::new(started_uploads.clone()))?;
        registry.register(Box::new(finished_uploads.clone()))?;
        registry.register(Box::new(active_uploads.clone()))?;
        registry.register(Box::new(upload_sizes.clone()))?;
        registry.register(Box::new(terminated_uploads.clone()))?;
        registry.register(Box::new(found_errors.clone()))?;

        Ok(Self {
            started_uploads,
            finished_uploads,
            active_uploads,
            upload_sizes,
            terminated_uploads,
            found_errors,
            registry,
        })
    }
}
