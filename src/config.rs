//! Serde-based configuration parsing for plugins
//!
//! Provides a custom deserializer that converts a flat string map (`PluginConfig`)
//! into typed configuration structs using serde.

use std::fmt;

use abi_stable::std_types::{RString, Tuple2};
use serde::Deserialize;
use serde::de::{self, DeserializeSeed, IntoDeserializer, MapAccess, Visitor};

use crate::{PluginConfig, PluginError, PluginResult};

/// Parse a PluginConfig into a typed config struct.
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
/// }
///
/// impl Default for Config {
///     fn default() -> Self {
///         Config {
///             format: "%H:%M".to_string(),
///             interval: 1000,
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
    let deserializer = ConfigDeserializer { config };
    match T::deserialize(deserializer) {
        Ok(value) => PluginResult::ROk(value),
        Err(e) => PluginResult::RErr(PluginError::new(e.to_string())),
    }
}

/// Error type for config deserialization
#[derive(Debug)]
pub struct ConfigError {
    message: String,
}

impl ConfigError {
    fn new<S>(message: S) -> Self
    where
        S: Into<String>,
    {
        ConfigError {
            message: message.into(),
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ConfigError {}

impl de::Error for ConfigError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        ConfigError::new(msg.to_string())
    }
}

/// Custom deserializer for PluginConfig -> T
struct ConfigDeserializer<'a> {
    config: &'a PluginConfig,
}

impl<'de, 'a> de::Deserializer<'de> for ConfigDeserializer<'a> {
    type Error = ConfigError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_map(ConfigMapAccess::new(self.config))
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    // Forward all other deserialize methods to deserialize_any
    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct enum identifier ignored_any
    }
}

/// Map access implementation for iterating over PluginConfig entries
struct ConfigMapAccess<'a> {
    iter: std::vec::IntoIter<Tuple2<&'a RString, &'a RString>>,
    current_value: Option<&'a RString>,
}

impl<'a> ConfigMapAccess<'a> {
    fn new(config: &'a PluginConfig) -> Self {
        let entries: Vec<_> = config.iter().collect();
        ConfigMapAccess {
            iter: entries.into_iter(),
            current_value: None,
        }
    }
}

impl<'de, 'a> MapAccess<'de> for ConfigMapAccess<'a> {
    type Error = ConfigError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(Tuple2(key, value)) => {
                self.current_value = Some(value);
                seed.deserialize(StringDeserializer(key.as_str())).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        match self.current_value.take() {
            Some(value) => seed.deserialize(ValueDeserializer(value.as_str())),
            None => Err(ConfigError::new("value missing for key")),
        }
    }
}

/// Deserializer for string keys
struct StringDeserializer<'a>(&'a str);

impl<'de, 'a> de::Deserializer<'de> for StringDeserializer<'a> {
    type Error = ConfigError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_str(self.0)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_str(self.0)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.0.to_owned())
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_str(self.0)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum ignored_any
    }
}

/// Deserializer for string values that handles type coercion
struct ValueDeserializer<'a>(&'a str);

impl<'de, 'a> de::Deserializer<'de> for ValueDeserializer<'a> {
    type Error = ConfigError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_str(self.0)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value = match self.0.to_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => true,
            "false" | "0" | "no" | "off" => false,
            _ => {
                return Err(ConfigError::new(format!(
                    "cannot parse '{}' as boolean",
                    self.0
                )));
            }
        };
        visitor.visit_bool(value)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value: i8 = self
            .0
            .parse()
            .map_err(|_| ConfigError::new(format!("cannot parse '{}' as i8", self.0)))?;
        visitor.visit_i8(value)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value: i16 = self
            .0
            .parse()
            .map_err(|_| ConfigError::new(format!("cannot parse '{}' as i16", self.0)))?;
        visitor.visit_i16(value)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value: i32 = self
            .0
            .parse()
            .map_err(|_| ConfigError::new(format!("cannot parse '{}' as i32", self.0)))?;
        visitor.visit_i32(value)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value: i64 = self
            .0
            .parse()
            .map_err(|_| ConfigError::new(format!("cannot parse '{}' as i64", self.0)))?;
        visitor.visit_i64(value)
    }

    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value: i128 = self
            .0
            .parse()
            .map_err(|_| ConfigError::new(format!("cannot parse '{}' as i128", self.0)))?;
        visitor.visit_i128(value)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value: u8 = self
            .0
            .parse()
            .map_err(|_| ConfigError::new(format!("cannot parse '{}' as u8", self.0)))?;
        visitor.visit_u8(value)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value: u16 = self
            .0
            .parse()
            .map_err(|_| ConfigError::new(format!("cannot parse '{}' as u16", self.0)))?;
        visitor.visit_u16(value)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value: u32 = self
            .0
            .parse()
            .map_err(|_| ConfigError::new(format!("cannot parse '{}' as u32", self.0)))?;
        visitor.visit_u32(value)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value: u64 = self
            .0
            .parse()
            .map_err(|_| ConfigError::new(format!("cannot parse '{}' as u64", self.0)))?;
        visitor.visit_u64(value)
    }

    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value: u128 = self
            .0
            .parse()
            .map_err(|_| ConfigError::new(format!("cannot parse '{}' as u128", self.0)))?;
        visitor.visit_u128(value)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value: f32 = self
            .0
            .parse()
            .map_err(|_| ConfigError::new(format!("cannot parse '{}' as f32", self.0)))?;
        visitor.visit_f32(value)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let value: f64 = self
            .0
            .parse()
            .map_err(|_| ConfigError::new(format!("cannot parse '{}' as f64", self.0)))?;
        visitor.visit_f64(value)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_str(self.0)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.0.to_owned())
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_enum(self.0.into_deserializer())
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_str(self.0)
    }

    serde::forward_to_deserialize_any! {
        char bytes byte_buf seq tuple tuple_struct map struct ignored_any
    }
}
