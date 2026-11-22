use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogRecord<'a> {
    pub timestamp_ms: u128,
    pub session_generation: u64,
    pub level: LogLevel,
    pub category: &'a str,
    pub event: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<&'a str>,
}

pub struct StructuredLogger {
    path: Option<PathBuf>,
    session_generation: u64,
    lock: Mutex<()>,
}

impl StructuredLogger {
    pub fn new(path: Option<PathBuf>, session_generation: u64) -> Self {
        Self {
            path,
            session_generation,
            lock: Mutex::new(()),
        }
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn log(
        &self,
        level: LogLevel,
        category: &str,
        event: &str,
        duration_ms: Option<u64>,
        detail: Option<&str>,
    ) {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let record = LogRecord {
            timestamp_ms,
            session_generation: self.session_generation,
            level,
            category,
            event,
            duration_ms,
            detail,
        };
        let Ok(line) = serde_json::to_string(&record) else {
            return;
        };
        let Some(path) = &self.path else {
            return;
        };
        let _guard = self.lock.lock();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(file, "{line}");
        }
    }

    pub fn info(&self, category: &str, event: &str) {
        self.log(LogLevel::Info, category, event, None, None);
    }

    pub fn timing(&self, category: &str, event: &str, duration_ms: u64) {
        self.log(LogLevel::Info, category, event, Some(duration_ms), None);
    }

    pub fn warn(&self, category: &str, event: &str, detail: &str) {
        self.log(LogLevel::Warn, category, event, None, Some(detail));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_valid_json_lines() {
        let dir = std::env::temp_dir().join(format!("stellaria-log-test-{}", std::process::id()));
        let file = dir.join("shell.jsonl");
        let _ = std::fs::remove_file(&file);

        let logger = StructuredLogger::new(Some(file.clone()), 7);
        logger.timing("startup", "rust_setup_complete", 42);

        let content = std::fs::read_to_string(&file).expect("log file written");
        let value: serde_json::Value = serde_json::from_str(content.trim()).expect("valid json");
        assert_eq!(value["sessionGeneration"], 7);
        assert_eq!(value["category"], "startup");
        assert_eq!(value["event"], "rust_setup_complete");
        assert_eq!(value["durationMs"], 42);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
