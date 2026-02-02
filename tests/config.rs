use abi_stable::std_types::RHashMap;
use serde::Deserialize;
use verandah_plugin_api::{parse_config, PluginError, PluginResult};

#[test]
fn test_parse_string_config() -> Result<(), PluginError> {
    #[derive(Deserialize, Debug, PartialEq)]
    struct Config {
        name: String,
    }

    let mut map = RHashMap::new();
    map.insert("name".into(), "test".into());

    let result: Config = parse_config(&map).into_result()?;
    assert_eq!(
        result,
        Config {
            name: "test".to_string()
        }
    );
    Ok(())
}

#[test]
fn test_parse_numeric_config() -> Result<(), PluginError> {
    #[derive(Deserialize, Debug, PartialEq)]
    struct Config {
        interval: u64,
        count: i32,
        rate: f64,
    }

    let mut map = RHashMap::new();
    map.insert("interval".into(), "1000".into());
    map.insert("count".into(), "-5".into());
    map.insert("rate".into(), "3.14".into());

    let result: Config = parse_config(&map).into_result()?;
    assert_eq!(
        result,
        Config {
            interval: 1000,
            count: -5,
            rate: 3.14
        }
    );
    Ok(())
}

#[test]
fn test_parse_bool_config() -> Result<(), PluginError> {
    #[derive(Deserialize, Debug, PartialEq)]
    struct Config {
        enabled: bool,
        disabled: bool,
    }

    let mut map = RHashMap::new();
    map.insert("enabled".into(), "true".into());
    map.insert("disabled".into(), "0".into());

    let result: Config = parse_config(&map).into_result()?;
    assert_eq!(
        result,
        Config {
            enabled: true,
            disabled: false
        }
    );
    Ok(())
}

#[test]
fn test_parse_default_config() -> Result<(), PluginError> {
    #[derive(Deserialize, Debug, PartialEq)]
    #[serde(default)]
    struct Config {
        name: String,
        interval: u64,
    }

    impl Default for Config {
        fn default() -> Self {
            Config {
                name: "default_name".to_string(),
                interval: 1000,
            }
        }
    }

    let mut map = RHashMap::new();
    map.insert("name".into(), "custom".into());

    let result: Config = parse_config(&map).into_result()?;
    assert_eq!(
        result,
        Config {
            name: "custom".to_string(),
            interval: 1000, // default
        }
    );
    Ok(())
}

#[test]
fn test_parse_enum_config() -> Result<(), PluginError> {
    #[derive(Deserialize, Debug, PartialEq)]
    #[serde(rename_all = "lowercase")]
    enum TargetType {
        Source,
        Sink,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct Config {
        target_type: TargetType,
    }

    let mut map = RHashMap::new();
    map.insert("target_type".into(), "sink".into());

    let result: Config = parse_config(&map).into_result()?;
    assert_eq!(
        result,
        Config {
            target_type: TargetType::Sink
        }
    );
    Ok(())
}

#[test]
fn test_parse_renamed_field() -> Result<(), PluginError> {
    #[derive(Deserialize, Debug, PartialEq)]
    struct Config {
        #[serde(rename = "type")]
        target_type: String,
    }

    let mut map = RHashMap::new();
    map.insert("type".into(), "source".into());

    let result: Config = parse_config(&map).into_result()?;
    assert_eq!(
        result,
        Config {
            target_type: "source".to_string()
        }
    );
    Ok(())
}

#[test]
fn test_invalid_numeric_fails() {
    #[derive(Deserialize, Debug)]
    struct Config {
        interval: u64,
    }

    let mut map = RHashMap::new();
    map.insert("interval".into(), "not_a_number".into());

    let result: PluginResult<Config> = parse_config(&map);
    assert!(result.is_err());
}
