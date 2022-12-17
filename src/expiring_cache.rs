use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub(crate) struct ExpiringValue<T> {
    pub value: T,
    pub added_at: std::time::Instant,
    pub expires_in_ms: Option<Duration>,
}

impl<T> ExpiringValue<T> {
    pub fn new(value: T, expires_in_ms: Option<Duration>) -> Self {
        Self {
            value,
            expires_in_ms,
            added_at: Instant::now(),
        }
    }
    pub fn create_non_expiring(val: T) -> Self {
        Self::new(val, None)
    }

    pub fn create_expiring(val: T, expire_in_ms: Duration) -> Self {
        Self::new(val, Some(expire_in_ms))
    }

    pub fn has_expired(&self) -> bool {
        match self.expires_in_ms {
            None => false,
            Some(duration) => self.added_at.elapsed() > duration,
        }
    }
}
