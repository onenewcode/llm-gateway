//! CLI 输出格式化

use crate::event::RoutingEvent;
use chrono::{DateTime, Local};

/// 输出格式枚举
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OutputFormat {
    #[default]
    Table,
    Json,
    Csv,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "table" => Ok(OutputFormat::Table),
            "json" => Ok(OutputFormat::Json),
            "csv" => Ok(OutputFormat::Csv),
            _ => Err(format!("Unknown format: {}", s)),
        }
    }
}

/// 格式化事件为输出字符串
pub fn format_events(events: &[RoutingEvent], format: OutputFormat) -> String {
    match format {
        OutputFormat::Table => format_table(events),
        OutputFormat::Json => format_json(events),
        OutputFormat::Csv => format_csv(events),
    }
}

fn format_table(events: &[RoutingEvent]) -> String {
    if events.is_empty() {
        return "No events found.".to_string();
    }

    let mut output = String::new();

    // Header
    output.push_str(&format!(
        "{:<22} {:<15} {:<15} {:>10} {:>8}\n",
        "Time", "Model", "Backend", "Duration", "Success"
    ));
    output.push_str(&"─".repeat(70));
    output.push('\n');

    // Rows
    for event in events {
        let timestamp: DateTime<Local> = DateTime::from_timestamp_millis(event.timestamp)
            .unwrap_or_default()
            .with_timezone(&Local);

        let success_mark = if event.success { "✓" } else { "✗" };

        output.push_str(&format!(
            "{:<22} {:<15} {:<15} {:>8}ms {:>8}\n",
            timestamp.format("%Y-%m-%d %H:%M:%S"),
            truncate(&event.model, 14),
            truncate(&event.backend, 14),
            event.duration_ms,
            success_mark
        ));
    }

    output
}

fn format_json(events: &[RoutingEvent]) -> String {
    use serde::Serialize;

    #[derive(Serialize)]
    struct EventJson {
        timestamp: String,
        model: String,
        backend: String,
        duration_ms: i64,
        success: bool,
        client: String,
    }

    let json_events: Vec<EventJson> = events
        .iter()
        .map(|e| {
            let timestamp: DateTime<Local> = DateTime::from_timestamp_millis(e.timestamp)
                .unwrap_or_default()
                .with_timezone(&Local);

            EventJson {
                timestamp: timestamp.to_rfc3339(),
                model: e.model.clone(),
                backend: e.backend.clone(),
                duration_ms: e.duration_ms,
                success: e.success,
                client: format!(
                    "{}.{}.{}.{}:{}",
                    (e.remote_addr >> 24) & 0xFF,
                    (e.remote_addr >> 16) & 0xFF,
                    (e.remote_addr >> 8) & 0xFF,
                    e.remote_addr & 0xFF,
                    e.remote_port
                ),
            }
        })
        .collect();

    serde_json::to_string_pretty(&json_events).unwrap_or_default()
}

fn format_csv(events: &[RoutingEvent]) -> String {
    let mut output = String::new();

    // Header
    output.push_str("timestamp,model,backend,duration_ms,success,client\n");

    // Rows
    for event in events {
        let timestamp: DateTime<Local> = DateTime::from_timestamp_millis(event.timestamp)
            .unwrap_or_default()
            .with_timezone(&Local);

        let client = format!(
            "{}.{}.{}.{}:{}",
            (event.remote_addr >> 24) & 0xFF,
            (event.remote_addr >> 16) & 0xFF,
            (event.remote_addr >> 8) & 0xFF,
            event.remote_addr & 0xFF,
            event.remote_port
        );

        output.push_str(&format!(
            "{},{},{},{},{},{}\n",
            timestamp.to_rfc3339(),
            event.model,
            event.backend,
            event.duration_ms,
            event.success,
            client
        ));
    }

    output
}

fn truncate(s: &str, max_len: usize) -> &str {
    if s.len() > max_len { &s[..max_len] } else { s }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::RoutingEvent;

    fn create_test_event(model: &str, backend: &str, success: bool) -> RoutingEvent {
        RoutingEvent::builder(1609459200000, 9000) // 2021-01-01 00:00:00 UTC
            .remote_addr_raw(0x7f000001, 8080) // 127.0.0.1
            .model(model)
            .backend(backend)
            .success(success)
            .duration_ms(150)
            .method("POST")
            .path("/v1/chat/completions")
            .routing_path("input->backend")
            .build()
    }

    #[test]
    fn test_output_format_from_str() {
        assert_eq!(
            "table".parse::<OutputFormat>().unwrap(),
            OutputFormat::Table
        );
        assert_eq!("json".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
        assert_eq!("csv".parse::<OutputFormat>().unwrap(), OutputFormat::Csv);
        assert_eq!(
            "TABLE".parse::<OutputFormat>().unwrap(),
            OutputFormat::Table
        );
        assert_eq!("JSON".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
    }

    #[test]
    fn test_output_format_from_str_invalid() {
        assert!("invalid".parse::<OutputFormat>().is_err());
        assert!("".parse::<OutputFormat>().is_err());
    }

    #[test]
    fn test_format_table_empty() {
        let result = format_table(&[]);
        assert_eq!(result, "No events found.");
    }

    #[test]
    fn test_format_table_single_event() {
        let events = vec![create_test_event("qwen-35b", "backend-1", true)];
        let result = format_table(&events);

        assert!(result.contains("Time"));
        assert!(result.contains("Model"));
        assert!(result.contains("Backend"));
        assert!(result.contains("Duration"));
        assert!(result.contains("Success"));
        assert!(result.contains("qwen-35b"));
        assert!(result.contains("backend-1"));
        assert!(result.contains("✓"));
    }

    #[test]
    fn test_format_table_failed_event() {
        let events = vec![create_test_event("qwen-35b", "backend-1", false)];
        let result = format_table(&events);

        assert!(result.contains("✗"));
    }

    #[test]
    fn test_format_table_multiple_events() {
        let events = vec![
            create_test_event("qwen-35b", "backend-1", true),
            create_test_event("qwen-72b", "backend-2", false),
        ];
        let result = format_table(&events);

        assert!(result.contains("qwen-35b"));
        assert!(result.contains("qwen-72b"));
        assert!(result.contains("backend-1"));
        assert!(result.contains("backend-2"));
    }

    #[test]
    fn test_format_json_empty() {
        let result = format_json(&[]);
        assert_eq!(result, "[]");
    }

    #[test]
    fn test_format_json_single_event() {
        let events = vec![create_test_event("qwen-35b", "backend-1", true)];
        let result = format_json(&events);

        assert!(result.contains("qwen-35b"));
        assert!(result.contains("backend-1"));
        assert!(result.contains("127.0.0.1:8080"));
        assert!(result.contains("\"success\": true"));
    }

    #[test]
    fn test_format_json_multiple_events() {
        let events = vec![
            create_test_event("qwen-35b", "backend-1", true),
            create_test_event("qwen-72b", "backend-2", false),
        ];
        let result = format_json(&events);

        assert!(result.contains("qwen-35b"));
        assert!(result.contains("qwen-72b"));
        assert!(result.contains("\"success\": true"));
        assert!(result.contains("\"success\": false"));
    }

    #[test]
    fn test_format_csv_empty() {
        let result = format_csv(&[]);
        assert_eq!(
            result,
            "timestamp,model,backend,duration_ms,success,client\n"
        );
    }

    #[test]
    fn test_format_csv_single_event() {
        let events = vec![create_test_event("qwen-35b", "backend-1", true)];
        let result = format_csv(&events);

        assert!(result.contains("timestamp,model,backend,duration_ms,success,client"));
        assert!(result.contains("qwen-35b"));
        assert!(result.contains("backend-1"));
        assert!(result.contains("127.0.0.1:8080"));
        assert!(result.contains("true"));
    }

    #[test]
    fn test_format_csv_multiple_events() {
        let events = vec![
            create_test_event("qwen-35b", "backend-1", true),
            create_test_event("qwen-72b", "backend-2", false),
        ];
        let result = format_csv(&events);

        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 3); // header + 2 data rows
        assert!(lines[0].contains("timestamp,model,backend"));
        assert!(lines[1].contains("qwen-35b"));
        assert!(lines[2].contains("qwen-72b"));
    }

    #[test]
    fn test_format_events_table() {
        let events = vec![create_test_event("test", "backend", true)];
        let result = format_events(&events, OutputFormat::Table);
        assert!(result.contains("test"));
    }

    #[test]
    fn test_format_events_json() {
        let events = vec![create_test_event("test", "backend", true)];
        let result = format_events(&events, OutputFormat::Json);
        assert!(result.contains("test"));
    }

    #[test]
    fn test_format_events_csv() {
        let events = vec![create_test_event("test", "backend", true)];
        let result = format_events(&events, OutputFormat::Csv);
        assert!(result.contains("test"));
    }

    #[test]
    fn test_truncate_short_string() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_long_string() {
        assert_eq!(truncate("hello world", 5), "hello");
    }

    #[test]
    fn test_truncate_exact_length() {
        assert_eq!(truncate("hello", 5), "hello");
    }
}
