use prometheus::{Histogram, IntCounter, IntGauge};

pub struct StartedUploads(pub IntCounter);
pub struct FinishedUploads(pub IntCounter);
pub struct ActiveUploads(pub IntGauge);

pub struct UploadSizes(pub Histogram);

pub struct TerminatedUploads(pub IntCounter);
