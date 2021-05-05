//! Configuration for `HttpClient`s.

use std::fmt::Debug;
use std::time::Duration;

/// Configuration for `HttpClient`s.
#[non_exhaustive]
#[derive(Clone)]
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
    /// TLS Configuration (Rustls)
    #[cfg(all(feature = "h1_client", feature = "rustls"))]
    pub tls_config: Option<std::sync::Arc<rustls_crate::ClientConfig>>,
    /// TLS Configuration (Native TLS)
    #[cfg(all(feature = "h1_client", feature = "native-tls", not(feature = "rustls")))]
    pub tls_config: Option<std::sync::Arc<async_native_tls::TlsConnector>>,
}

impl Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg_struct = f.debug_struct("Config");
        dbg_struct
            .field("http_keep_alive", &self.http_keep_alive)
            .field("tcp_no_delay", &self.tcp_no_delay)
            .field("timeout", &self.timeout);

        #[cfg(all(feature = "h1_client", feature = "rustls"))]
        {
            if self.tls_config.is_some() {
                dbg_struct.field("tls_config", &"Some(rustls::ClientConfig)");
            } else {
                dbg_struct.field("tls_config", &"None");
            }
        }
        #[cfg(all(feature = "h1_client", feature = "native-tls", not(feature = "rustls")))]
        {
            dbg_struct.field("tls_config", &self.tls_config);
        }

        dbg_struct.finish()
    }
}

impl Config {
    /// Construct new empty config.
    pub fn new() -> Self {
        Self {
            http_keep_alive: true,
            tcp_no_delay: false,
            timeout: Some(Duration::from_secs(60)),
            #[cfg(all(feature = "h1_client", any(feature = "rustls", feature = "native-tls")))]
            tls_config: None,
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

    /// Set TLS Configuration (Rustls)
    #[cfg(all(feature = "h1_client", feature = "rustls"))]
    pub fn set_tls_config(
        mut self,
        tls_config: Option<std::sync::Arc<rustls_crate::ClientConfig>>,
    ) -> Self {
        self.tls_config = tls_config;
        self
    }
    /// Set TLS Configuration (Native TLS)
    #[cfg(all(feature = "h1_client", feature = "native-tls", not(feature = "rustls")))]
    pub fn set_tls_config(
        mut self,
        tls_config: Option<std::sync::Arc<async_native_tls::TlsConnector>>,
    ) -> Self {
        self.tls_config = tls_config;
        self
    }
}
