use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const LOG_DIR_NAME: &str = "logs";
const LOG_FILE_NAME: &str = "wallpaper-engine.log";
const MAX_LOG_STRING_CHARS: usize = 300;
const MAX_LOG_ARRAY_ITEMS: usize = 25;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendLogEntry {
    #[serde(default = "default_level")]
    pub level: String,
    pub action: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub details: Option<Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LogRecord<'a> {
    timestamp_unix_ms: u128,
    level: &'a str,
    target: &'a str,
    action: &'a str,
    message: &'a str,
    details: Value,
}

pub fn log_path(data_dir: &Path) -> PathBuf {
    data_dir.join(LOG_DIR_NAME).join(LOG_FILE_NAME)
}

pub fn append_frontend_entry(path: &Path, entry: &FrontendLogEntry) -> io::Result<()> {
    append_event(
        path,
        normalize_level(&entry.level),
        "frontend",
        &entry.action,
        &entry.message,
        entry.details.clone().unwrap_or(Value::Null),
    )
}

pub fn append_event(
    path: &Path,
    level: &str,
    target: &str,
    action: &str,
    message: &str,
    details: Value,
) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let record = LogRecord {
        timestamp_unix_ms: timestamp_unix_ms(),
        level: normalize_level(level),
        target,
        action,
        message,
        details: sanitize_value(details),
    };
    let mut line = serde_json::to_vec(&record).map_err(io::Error::other)?;
    line.push(b'\n');

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(&line)
}

fn default_level() -> String {
    "info".into()
}

fn normalize_level(level: &str) -> &'static str {
    match level {
        "debug" => "debug",
        "warn" => "warn",
        "error" => "error",
        _ => "info",
    }
}

fn timestamp_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn sanitize_value(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(sanitize_object(map)),
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .take(MAX_LOG_ARRAY_ITEMS)
                .map(sanitize_value)
                .collect(),
        ),
        Value::String(value) => Value::String(truncate_log_string(value)),
        other => other,
    }
}

fn sanitize_object(map: Map<String, Value>) -> Map<String, Value> {
    map.into_iter()
        .map(|(key, value)| {
            let value = if is_sensitive_key(&key) {
                Value::String("[redacted]".into())
            } else {
                sanitize_value(value)
            };
            (key, value)
        })
        .collect()
}

fn is_sensitive_key(key: &str) -> bool {
    let normalized = key
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect::<String>();

    [
        "apikey",
        "anonkey",
        "authorization",
        "authtoken",
        "secret",
        "password",
        "token",
        "publishablekey",
        "postgresql",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn truncate_log_string(value: String) -> String {
    if value.chars().count() <= MAX_LOG_STRING_CHARS {
        return value;
    }

    let mut truncated = value.chars().take(MAX_LOG_STRING_CHARS).collect::<String>();
    truncated.push_str("...");
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn temp_log_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir()
            .join(format!("wallpaper-engine-log-{name}-{nanos}"))
            .join(LOG_FILE_NAME)
    }

    #[test]
    fn appends_json_lines_to_log_file() {
        let path = temp_log_path("append");

        append_event(&path, "info", "test", "first", "First event.", json!({}))
            .expect("first log event should append");
        append_event(&path, "warn", "test", "second", "Second event.", json!({}))
            .expect("second log event should append");

        let raw = fs::read_to_string(&path).expect("log file should exist");
        let lines = raw.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("\"action\":\"first\""));
        assert!(lines[1].contains("\"level\":\"warn\""));

        let _ = fs::remove_dir_all(path.parent().expect("log path has parent"));
    }

    #[test]
    fn redacts_sensitive_frontend_details() {
        let path = temp_log_path("redact");
        let entry = FrontendLogEntry {
            level: "info".into(),
            action: "settings.save".into(),
            message: "Saved settings.".into(),
            details: Some(json!({
                "apiKey": "should-not-appear",
                "nested": {
                    "accessToken": "also-secret",
                    "query": "nature"
                }
            })),
        };

        append_frontend_entry(&path, &entry).expect("frontend log event should append");

        let raw = fs::read_to_string(&path).expect("log file should exist");
        assert!(!raw.contains("should-not-appear"));
        assert!(!raw.contains("also-secret"));
        assert!(raw.contains("[redacted]"));
        assert!(raw.contains("nature"));

        let _ = fs::remove_dir_all(path.parent().expect("log path has parent"));
    }
}
