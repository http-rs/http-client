//! Configuration for `HttpClient`s.

use std::time::Duration;

/// Configuration for `HttpClient`s.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct Config {
    /// HTTP/1.1 `keep-alive` (connection pooling).
    ///
    /// Default: `true`.
    pub http_keep_alive: bool,
    /// TCP `NO_DELAY`.
    ///
    /// Default: `false`.
    pub tcp_no_delay: bool,
    /// Connection timeout duration.
    ///
    /// Default: `Some(Duration::from_secs(60))`.
    pub timeout: Option<Duration>,
}

impl Config {
    /// Construct new empty config.
    pub fn new() -> Self {
        Self {
            http_keep_alive: true,
            tcp_no_delay: false,
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
    /// Set HTTP/1.1 `keep-alive` (connection pooling).
    pub fn set_http_keep_alive(mut self, keep_alive: bool) -> Self {
        self.http_keep_alive = keep_alive;
        self
    }

    /// Set TCP `NO_DELAY`.
    pub fn set_tcp_no_delay(mut self, no_delay: bool) -> Self {
        self.tcp_no_delay = no_delay;
        self
    }

    /// Set connection timeout duration.
    pub fn set_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = timeout;
        self
    }
}
