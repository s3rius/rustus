use std::fmt::Display;

pub trait MonadLogger: Sized {
    #[must_use]
    fn _should_log(&self) -> bool;

    #[must_use]
    fn mlog_err(self, msg: &str) -> Self {
        if self._should_log() {
            tracing::error!(msg);
        }
        self
    }

    #[must_use]
    fn mlog_warn(self, msg: &str) -> Self {
        if self._should_log() {
            tracing::warn!(msg);
        }
        self
    }

    #[must_use]
    #[allow(unused_variables)]
    fn mlog_dbg(self, msg: &str) -> Self {
        #[cfg(debug_assertions)]
        if self._should_log() {
            tracing::debug!(msg);
        }
        self
    }
}

impl<T, E: Display> MonadLogger for Result<T, E> {
    fn _should_log(&self) -> bool {
        self.is_err()
    }
}

impl<T> MonadLogger for Option<T> {
    fn _should_log(&self) -> bool {
        self.is_none()
    }
}
