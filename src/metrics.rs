use crate::errors::RustusResult;

#[derive(Clone)]
pub struct StartedUploads {
    pub counter: prometheus::IntCounter,
}

impl StartedUploads {
    pub fn new() -> RustusResult<Self> {
        Ok(Self {
            counter: prometheus::IntCounter::new("started_uploads", "Number of created uploads")?,
        })
    }
}
#[derive(Clone)]
pub struct FinishedUploads {
    pub counter: prometheus::IntCounter,
}

impl FinishedUploads {
    pub fn new() -> RustusResult<Self> {
        Ok(Self {
            counter: prometheus::IntCounter::new("finished_uploads", "Number of finished uploads")?,
        })
    }
}

#[derive(Clone)]
pub struct ActiveUploads {
    pub gauge: prometheus::IntGauge,
}
impl ActiveUploads {
    pub fn new() -> RustusResult<Self> {
        Ok(Self {
            gauge: prometheus::IntGauge::new("active_uploads", "Number of active file uploads")?,
        })
    }
}

#[derive(Clone)]
pub struct UploadSizes {
    pub hist: prometheus::Histogram,
}

impl UploadSizes {
    pub fn new() -> RustusResult<Self> {
        Ok(Self {
            hist: prometheus::Histogram::with_opts(
                prometheus::HistogramOpts::new("uploads_sizes", "Size of uploaded files in bytes")
                    .buckets(prometheus::exponential_buckets(2., 2., 40)?),
            )?,
        })
    }
}

#[derive(Clone)]
pub struct TerminatedUploads {
    pub counter: prometheus::IntCounter,
}

impl TerminatedUploads {
    pub fn new() -> RustusResult<Self> {
        Ok(Self {
            counter: prometheus::IntCounter::new(
                "terminated_uploads",
                "Number of terminated uploads",
            )?,
        })
    }
}
