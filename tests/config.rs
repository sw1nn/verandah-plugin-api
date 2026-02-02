use abi_stable::std_types::RString;
use serde::Deserialize;
use verandah_plugin_api::{PluginError, PluginResult, parse_config};

#[test]
fn test_parse_string_config() -> Result<(), PluginError> {
    #[derive(Deserialize, Debug, PartialEq)]
    struct Config {
        name: String,
    }

    let toml = RString::from("name = \"test\"");

    let result: Config = parse_config(&toml).into_result()?;
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

    let toml = RString::from(
        r#"
interval = 1000
count = -5
rate = 3.14
"#,
    );

    let result: Config = parse_config(&toml).into_result()?;
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

    let toml = RString::from(
        r#"
enabled = true
disabled = false
"#,
    );

    let result: Config = parse_config(&toml).into_result()?;
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

    let toml = RString::from("name = \"custom\"");

    let result: Config = parse_config(&toml).into_result()?;
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

    let toml = RString::from("target_type = \"sink\"");

    let result: Config = parse_config(&toml).into_result()?;
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

    let toml = RString::from("type = \"source\"");

    let result: Config = parse_config(&toml).into_result()?;
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

    let toml = RString::from("interval = \"not_a_number\"");

    let result: PluginResult<Config> = parse_config(&toml);
    assert!(result.is_err());
}

#[test]
fn test_nested_config() -> Result<(), PluginError> {
    #[derive(Deserialize, Debug, PartialEq)]
    struct Colors {
        fg: String,
        bg: String,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct Config {
        name: String,
        colors: Colors,
    }

    let toml = RString::from(
        r##"
name = "my-widget"
colors = {fg = "#ffffff", bg = "#000000"}
"##,
    );

    let result: Config = parse_config(&toml).into_result()?;
    assert_eq!(
        result,
        Config {
            name: "my-widget".to_string(),
            colors: Colors {
                fg: "#ffffff".to_string(),
                bg: "#000000".to_string(),
            }
        }
    );
    Ok(())
}

#[test]
fn test_nested_table_config() -> Result<(), PluginError> {
    #[derive(Deserialize, Debug, PartialEq)]
    struct Colors {
        fg: String,
        bg: String,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct Config {
        name: String,
        colors: Colors,
    }

    let toml = RString::from(
        r##"
name = "my-widget"

[colors]
fg = "#ffffff"
bg = "#000000"
"##,
    );

    let result: Config = parse_config(&toml).into_result()?;
    assert_eq!(
        result,
        Config {
            name: "my-widget".to_string(),
            colors: Colors {
                fg: "#ffffff".to_string(),
                bg: "#000000".to_string(),
            }
        }
    );
    Ok(())
}

#[test]
fn test_array_config() -> Result<(), PluginError> {
    #[derive(Deserialize, Debug, PartialEq)]
    struct Config {
        tags: Vec<String>,
    }

    let toml = RString::from(r#"tags = ["a", "b", "c"]"#);

    let result: Config = parse_config(&toml).into_result()?;
    assert_eq!(
        result,
        Config {
            tags: vec!["a".to_string(), "b".to_string(), "c".to_string()]
        }
    );
    Ok(())
}
