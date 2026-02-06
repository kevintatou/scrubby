use std::fs;
use std::path::Path;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Config {
    pub stable_placeholders: Option<bool>,
    pub json_report: Option<bool>,
    pub interval_ms: Option<u64>,
}

#[derive(Debug)]
pub struct ConfigError {
    pub message: String,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ConfigError {}

pub fn load_config(path: &Path) -> Result<Config, ConfigError> {
    let content = fs::read_to_string(path).map_err(|e| ConfigError {
        message: format!("Failed to read config: {}", e),
    })?;

    parse_config(&content)
}

fn parse_config(input: &str) -> Result<Config, ConfigError> {
    let mut cfg = Config::default();
    for (i, raw) in input.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.splitn(2, '=');
        let key = parts.next().unwrap().trim();
        let value = parts.next().unwrap_or("").trim();
        if value.is_empty() {
            return Err(ConfigError {
                message: format!("Config parse error at line {}: missing value", i + 1),
            });
        }
        match key {
            "stable_placeholders" => {
                cfg.stable_placeholders = Some(parse_bool(value, i + 1)?);
            }
            "json_report" => {
                cfg.json_report = Some(parse_bool(value, i + 1)?);
            }
            "interval_ms" => {
                cfg.interval_ms = Some(parse_u64(value, i + 1)?);
            }
            _ => {
                return Err(ConfigError {
                    message: format!("Unknown config key '{}' at line {}", key, i + 1),
                });
            }
        }
    }
    Ok(cfg)
}

fn parse_bool(value: &str, line: usize) -> Result<bool, ConfigError> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Ok(true),
        "false" | "0" | "no" => Ok(false),
        _ => Err(ConfigError {
            message: format!("Invalid boolean '{}' at line {}", value, line),
        }),
    }
}

fn parse_u64(value: &str, line: usize) -> Result<u64, ConfigError> {
    value.parse::<u64>().map_err(|_| ConfigError {
        message: format!("Invalid integer '{}' at line {}", value, line),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_config() {
        let cfg =
            parse_config("stable_placeholders=true\njson_report=false\ninterval_ms=500\n").unwrap();
        assert_eq!(cfg.stable_placeholders, Some(true));
        assert_eq!(cfg.json_report, Some(false));
        assert_eq!(cfg.interval_ms, Some(500));
    }
}
