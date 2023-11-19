use std::fmt::Display;

pub trait MonadLogger {
    fn mlog(self, level: log::Level, msg: &str) -> Self;
    fn mlog_err(self, msg: &str) -> Self;
    fn mlog_warn(self, msg: &str) -> Self;
    fn mlog_dbg(self, msg: &str) -> Self;
}

impl<T, E: Display> MonadLogger for Result<T, E> {
    fn mlog(self, level: log::Level, msg: &str) -> Self {
        if let Err(err) = &self {
            log::log!(level, "{msg}: {err}");
        }
        self
    }

    fn mlog_err(self, msg: &str) -> Self {
        self.mlog(log::Level::Error, msg)
    }

    fn mlog_warn(self, msg: &str) -> Self {
        self.mlog(log::Level::Warn, msg)
    }

    fn mlog_dbg(self, msg: &str) -> Self {
        self.mlog(log::Level::Debug, msg)
    }
}

impl<T> MonadLogger for Option<T> {
    fn mlog(self, level: log::Level, msg: &str) -> Self {
        if self.is_none() {
            log::log!(level, "{msg}: The value is None");
        }
        self
    }
    fn mlog_err(self, msg: &str) -> Self {
        self.mlog(log::Level::Error, msg)
    }

    fn mlog_warn(self, msg: &str) -> Self {
        self.mlog(log::Level::Warn, msg)
    }

    fn mlog_dbg(self, msg: &str) -> Self {
        self.mlog(log::Level::Debug, msg)
    }
}
