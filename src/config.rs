//! Configuration for `HttpClient`s.

/// Configuration for `HttpClient`s.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct Config {
    /// TCP `NO_DELAY`.
    pub no_delay: Option<Option<bool>>,
    /// Connection timeout duration.
    pub timeout: Option<Option<u64>>,
}

impl Config {
    /// Construct new empty config.
    pub fn new() -> Self {
        Self {
            no_delay: None,
            timeout: None,
        }
    }

    /// Construct a new chainable configuration builder.
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::new()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl From<ConfigBuilder> for Config {
    fn from(builder: ConfigBuilder) -> Self {
        builder.build()
    }
}

/// A chainable builder for `Config`.
#[derive(Clone, Debug)]
pub struct ConfigBuilder(Config);

impl ConfigBuilder {
    /// Construct a new chainable configuration builder.
    pub fn new() -> Self {
        Self(Config::new())
    }

    /// Consume the builder, returning the resulting `Config`.
    pub fn build(self) -> Config {
        self.0
    }

    /// Set TCP `NO_DELAY`.
    pub fn set_no_delay(mut self, no_delay: bool) -> Self {
        self.0.no_delay = Some(Some(no_delay));
        self
    } 

    /// Unset TCP `NO_DELAY` (use default).
    pub fn unset_no_delay(mut self) -> Self {
        self.0.no_delay = Some(None);
        self
    }

    /// Set connection timeout duration.
    pub fn set_timeout(mut self, timeout: u64) -> Self {
        self.0.timeout = Some(Some(timeout));
        self
    } 

    /// Unset onnection timeout duration (use default).
    pub fn unset_timeout(mut self) -> Self {
        self.0.timeout = Some(None);
        self
    }
}

impl AsRef<Config> for ConfigBuilder {
    fn as_ref(&self) -> &Config {
        &self.0
    }
}

impl AsMut<Config> for ConfigBuilder {
    fn as_mut(&mut self) -> &mut Config {
        &mut self.0
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}



