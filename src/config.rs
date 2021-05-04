//! Configuration for `HttpClient`s.

use std::time::Duration;

/// Configuration for `HttpClient`s.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct Config {
    /// TCP `NO_DELAY`.
    ///
    /// Default: `false`.
    pub no_delay: bool,
    /// Connection timeout duration.
    ///
    /// Default: `Some(Duration::from_secs(60))`.
    pub timeout: Option<Duration>,
}

impl Config {
    /// Construct new empty config.
    pub fn new() -> Self {
        Self {
            no_delay: false,
            timeout: Some(Duration::from_secs(60)),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    /// Set TCP `NO_DELAY`.
    pub fn set_no_delay(mut self, no_delay: bool) -> Self {
        self.no_delay = no_delay;
        self
    }

    /// Set connection timeout duration.
    pub fn set_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = timeout;
        self
    }
}
