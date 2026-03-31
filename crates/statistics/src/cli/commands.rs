//! 命令解析与分发

use crate::cli::formatter::OutputFormat;
use crate::query::{AggQuery, EventFilter};
use chrono::Utc;
use humantime::parse_duration;

/// 解析后的命令
#[derive(Debug, Clone)]
pub enum Command {
    Query {
        filter: EventFilter,
        format: OutputFormat,
    },
    Stats {
        query: AggQuery,
    },
    Models {
        sort: String,
        format: OutputFormat,
    },
    Backends {
        sort: String,
        format: OutputFormat,
    },
    Recent {
        limit: usize,
    },
    Detail {
        index: usize,
    },
    Help,
    Exit,
    Unknown(String),
}

impl Command {
    /// 从输入行解析命令
    pub fn parse(input: &str) -> Self {
        let parts: Vec<&str> = input.split_whitespace().collect();

        if parts.is_empty() {
            return Command::Unknown(String::new());
        }

        match parts[0].to_lowercase().as_str() {
            "query" => Self::parse_query(&parts[1..]),
            "stats" => Self::parse_stats(&parts[1..]),
            "models" => Self::parse_models(&parts[1..]),
            "backends" => Self::parse_backends(&parts[1..]),
            "recent" => Self::parse_recent(&parts[1..]),
            "detail" => Self::parse_detail(&parts[1..]),
            "help" | "?" => Command::Help,
            "exit" | "quit" | "q" => Command::Exit,
            _ => Command::Unknown(input.to_string()),
        }
    }

    fn parse_query(args: &[&str]) -> Self {
        let mut filter = EventFilter::default();
        let mut format = OutputFormat::Table;
        let mut last: Option<String> = None;

        let mut i = 0;
        while i < args.len() {
            match args[i] {
                "--last" if i + 1 < args.len() => {
                    last = Some(args[i + 1].to_string());
                    i += 2;
                }
                "--start" if i + 1 < args.len() => {
                    filter.start_time = parse_timestamp(args[i + 1]);
                    i += 2;
                }
                "--end" if i + 1 < args.len() => {
                    filter.end_time = parse_timestamp(args[i + 1]);
                    i += 2;
                }
                "--model" if i + 1 < args.len() => {
                    filter.model = Some(args[i + 1].to_string());
                    i += 2;
                }
                "--backend" if i + 1 < args.len() => {
                    filter.backend = Some(args[i + 1].to_string());
                    i += 2;
                }
                "--success" if i + 1 < args.len() => {
                    filter.success = args[i + 1].parse().ok();
                    i += 2;
                }
                "--limit" if i + 1 < args.len() => {
                    filter.limit = args[i + 1].parse().ok();
                    i += 2;
                }
                "--format" if i + 1 < args.len() => {
                    format = args[i + 1].parse().unwrap_or(OutputFormat::Table);
                    i += 2;
                }
                _ => i += 1,
            }
        }

        // Apply --last if specified
        if let Some(last_str) = last
            && let Ok(duration) = parse_duration(&last_str)
        {
            let now = Utc::now().timestamp_millis();
            let duration_ms = duration.as_millis() as i64;
            filter.start_time = Some(now - duration_ms);
            filter.end_time = Some(now);
        }

        // Default limit
        if filter.limit.is_none() {
            filter.limit = Some(100);
        }

        Command::Query { filter, format }
    }

    fn parse_stats(args: &[&str]) -> Self {
        let mut last = "1h".to_string();
        let mut window_size_secs = 3600u64;

        let mut i = 0;
        while i < args.len() {
            match args[i] {
                "--last" if i + 1 < args.len() => {
                    last = args[i + 1].to_string();
                    i += 2;
                }
                "--granularity" if i + 1 < args.len() => {
                    window_size_secs = crate::query::parse_time(args[i + 1]).unwrap_or(3600);
                    i += 2;
                }
                _ => i += 1,
            }
        }

        let now = Utc::now().timestamp_millis();
        let duration = parse_duration(&last).unwrap_or(std::time::Duration::from_secs(3600));
        let start = now - duration.as_millis() as i64;

        let window_size = std::num::NonZeroU64::new(window_size_secs)
            .unwrap_or_else(|| std::num::NonZeroU64::new(3600).unwrap());

        let query = AggQuery {
            start_time: start as u64,
            end_time: now as u64,
            window_size,
            model: None,
            backend: None,
        };

        Command::Stats { query }
    }

    fn parse_models(args: &[&str]) -> Self {
        let mut sort = "count".to_string();
        let mut format = OutputFormat::Table;

        let mut i = 0;
        while i < args.len() {
            match args[i] {
                "--sort" if i + 1 < args.len() => {
                    sort = args[i + 1].to_string();
                    i += 2;
                }
                "--format" if i + 1 < args.len() => {
                    format = args[i + 1].parse().unwrap_or(OutputFormat::Table);
                    i += 2;
                }
                _ => i += 1,
            }
        }

        Command::Models { sort, format }
    }

    fn parse_backends(args: &[&str]) -> Self {
        let mut sort = "count".to_string();
        let mut format = OutputFormat::Table;

        let mut i = 0;
        while i < args.len() {
            match args[i] {
                "--sort" if i + 1 < args.len() => {
                    sort = args[i + 1].to_string();
                    i += 2;
                }
                "--format" if i + 1 < args.len() => {
                    format = args[i + 1].parse().unwrap_or(OutputFormat::Table);
                    i += 2;
                }
                _ => i += 1,
            }
        }

        Command::Backends { sort, format }
    }

    fn parse_recent(args: &[&str]) -> Self {
        let mut limit = 20usize;

        let mut i = 0;
        while i < args.len() {
            match args[i] {
                "-n" if i + 1 < args.len() => {
                    limit = args[i + 1].parse().unwrap_or(20);
                    i += 2;
                }
                _ => i += 1,
            }
        }

        Command::Recent { limit }
    }

    fn parse_detail(args: &[&str]) -> Self {
        let index = args.first().and_then(|s| s.parse().ok()).unwrap_or(0);
        Command::Detail { index }
    }
}

fn parse_timestamp(s: &str) -> Option<i64> {
    // Try parsing as milliseconds timestamp first
    if let Ok(ts) = s.parse::<i64>() {
        return Some(ts);
    }

    // Try parsing as ISO8601
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(dt.timestamp_millis());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_help() {
        assert!(matches!(Command::parse("help"), Command::Help));
        assert!(matches!(Command::parse("?"), Command::Help));
    }

    #[test]
    fn test_parse_exit() {
        assert!(matches!(Command::parse("exit"), Command::Exit));
        assert!(matches!(Command::parse("quit"), Command::Exit));
        assert!(matches!(Command::parse("q"), Command::Exit));
    }

    #[test]
    fn test_parse_query_with_filters() {
        let cmd = Command::parse("query --last 1h --model qwen-35b --limit 50");
        assert!(matches!(cmd, Command::Query { .. }));
    }

    #[test]
    fn test_parse_recent() {
        let cmd = Command::parse("recent -n 30");
        assert!(matches!(cmd, Command::Recent { limit: 30 }));
    }

    #[test]
    fn test_parse_unknown_command() {
        let cmd = Command::parse("foobar");
        assert!(matches!(cmd, Command::Unknown(_)));
    }

    #[test]
    fn test_parse_empty_input() {
        let cmd = Command::parse("");
        assert!(matches!(cmd, Command::Unknown(s) if s.is_empty()));
    }

    #[test]
    fn test_parse_query_default_limit() {
        let cmd = Command::parse("query");
        if let Command::Query { filter, .. } = cmd {
            assert_eq!(filter.limit, Some(100));
        } else {
            panic!("Expected Query command");
        }
    }

    #[test]
    fn test_parse_query_with_model() {
        let cmd = Command::parse("query --model qwen-35b");
        if let Command::Query { filter, .. } = cmd {
            assert_eq!(filter.model, Some("qwen-35b".to_string()));
        } else {
            panic!("Expected Query command");
        }
    }

    #[test]
    fn test_parse_query_with_backend() {
        let cmd = Command::parse("query --backend sglang");
        if let Command::Query { filter, .. } = cmd {
            assert_eq!(filter.backend, Some("sglang".to_string()));
        } else {
            panic!("Expected Query command");
        }
    }

    #[test]
    fn test_parse_query_with_success() {
        let cmd = Command::parse("query --success true");
        if let Command::Query { filter, .. } = cmd {
            assert_eq!(filter.success, Some(true));
        } else {
            panic!("Expected Query command");
        }

        let cmd = Command::parse("query --success false");
        if let Command::Query { filter, .. } = cmd {
            assert_eq!(filter.success, Some(false));
        } else {
            panic!("Expected Query command");
        }
    }

    #[test]
    fn test_parse_query_with_format() {
        let cmd = Command::parse("query --format json");
        if let Command::Query { format, .. } = cmd {
            assert_eq!(format, OutputFormat::Json);
        } else {
            panic!("Expected Query command");
        }

        let cmd = Command::parse("query --format csv");
        if let Command::Query { format, .. } = cmd {
            assert_eq!(format, OutputFormat::Csv);
        } else {
            panic!("Expected Query command");
        }
    }

    #[test]
    fn test_parse_models_default() {
        let cmd = Command::parse("models");
        if let Command::Models { sort, format } = cmd {
            assert_eq!(sort, "count");
            assert_eq!(format, OutputFormat::Table);
        } else {
            panic!("Expected Models command");
        }
    }

    #[test]
    fn test_parse_models_with_sort() {
        let cmd = Command::parse("models --sort name");
        if let Command::Models { sort, .. } = cmd {
            assert_eq!(sort, "name");
        } else {
            panic!("Expected Models command");
        }
    }

    #[test]
    fn test_parse_models_with_format() {
        let cmd = Command::parse("models --format json");
        if let Command::Models { format, .. } = cmd {
            assert_eq!(format, OutputFormat::Json);
        } else {
            panic!("Expected Models command");
        }
    }

    #[test]
    fn test_parse_backends_default() {
        let cmd = Command::parse("backends");
        if let Command::Backends { sort, format } = cmd {
            assert_eq!(sort, "count");
            assert_eq!(format, OutputFormat::Table);
        } else {
            panic!("Expected Backends command");
        }
    }

    #[test]
    fn test_parse_backends_with_sort() {
        let cmd = Command::parse("backends --sort duration");
        if let Command::Backends { sort, .. } = cmd {
            assert_eq!(sort, "duration");
        } else {
            panic!("Expected Backends command");
        }
    }

    #[test]
    fn test_parse_recent_default() {
        let cmd = Command::parse("recent");
        if let Command::Recent { limit } = cmd {
            assert_eq!(limit, 20);
        } else {
            panic!("Expected Recent command");
        }
    }

    #[test]
    fn test_parse_detail_with_index() {
        let cmd = Command::parse("detail 5");
        if let Command::Detail { index } = cmd {
            assert_eq!(index, 5);
        } else {
            panic!("Expected Detail command");
        }
    }

    #[test]
    fn test_parse_detail_default_index() {
        let cmd = Command::parse("detail");
        if let Command::Detail { index } = cmd {
            assert_eq!(index, 0);
        } else {
            panic!("Expected Detail command");
        }
    }

    #[test]
    fn test_parse_stats_default() {
        let cmd = Command::parse("stats");
        assert!(matches!(cmd, Command::Stats { .. }));
    }

    #[test]
    fn test_parse_stats_with_granularity() {
        let cmd = Command::parse("stats --granularity 15m");
        assert!(matches!(cmd, Command::Stats { .. }));
    }

    #[test]
    fn test_parse_query_with_multiple_filters() {
        let cmd =
            Command::parse("query --model qwen-35b --backend sglang --success true --limit 50");
        if let Command::Query { filter, .. } = cmd {
            assert_eq!(filter.model, Some("qwen-35b".to_string()));
            assert_eq!(filter.backend, Some("sglang".to_string()));
            assert_eq!(filter.success, Some(true));
            assert_eq!(filter.limit, Some(50));
        } else {
            panic!("Expected Query command");
        }
    }

    #[test]
    fn test_parse_timestamp_millis() {
        let result = parse_timestamp("1609459200000");
        assert_eq!(result, Some(1609459200000i64));
    }

    #[test]
    fn test_parse_timestamp_iso8601() {
        let result = parse_timestamp("2021-01-01T00:00:00Z");
        assert_eq!(result, Some(1609459200000i64));
    }

    #[test]
    fn test_parse_timestamp_invalid() {
        let result = parse_timestamp("invalid");
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_case_insensitive() {
        assert!(matches!(Command::parse("HELP"), Command::Help));
        assert!(matches!(Command::parse("EXIT"), Command::Exit));
        assert!(matches!(Command::parse("Query"), Command::Query { .. }));
        assert!(matches!(Command::parse("STATS"), Command::Stats { .. }));
    }
}
