//! Configuration parsing for plugins
//!
//! Provides a simple function to parse TOML configuration strings
//! into typed configuration structs.

use serde::Deserialize;

use crate::{PluginConfig, PluginError, PluginResult};

/// Parse a PluginConfig (TOML string) into a typed config struct.
///
/// # Example
/// ```ignore
/// use serde::Deserialize;
/// use verandah_plugin_api::prelude::*;
///
/// #[derive(Deserialize)]
/// #[serde(default)]
/// struct Config {
///     format: String,
///     interval: u64,
///     colors: Colors,
/// }
///
/// #[derive(Deserialize, Default)]
/// struct Colors {
///     fg: String,
///     bg: String,
/// }
///
/// impl Default for Config {
///     fn default() -> Self {
///         Config {
///             format: "%H:%M".to_string(),
///             interval: 1000,
///             colors: Colors::default(),
///         }
///     }
/// }
///
/// fn init(&mut self, config: PluginConfig) -> PluginResult<()> {
///     let cfg: Config = parse_config(&config)?;
///     // use cfg...
///     PluginResult::ROk(())
/// }
/// ```
pub fn parse_config<T>(config: &PluginConfig) -> PluginResult<T>
where
    T: for<'de> Deserialize<'de>,
{
    match toml::from_str(config.as_str()) {
        Ok(value) => PluginResult::ROk(value),
        Err(e) => PluginResult::RErr(PluginError::new(e.to_string())),
    }
}
