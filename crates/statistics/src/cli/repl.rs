//! REPL 循环实现
//!
//! 提供交互式命令行界面，支持查询事件、查看统计、浏览模型和后端等功能

use chrono::{Local, TimeZone};
use rustyline::{DefaultEditor, Result as ReadlineResult};
use std::collections::HashMap;

use crate::cli::commands::Command;
use crate::cli::formatter::{OutputFormat, format_events};
use crate::config::StatisticsConfig;
use crate::event::RoutingEvent;
use crate::query::EventFilter;
use crate::store::StatsStoreManager;

/// REPL 应用状态
pub struct ReplApp {
    /// 事件存储管理器
    store: StatsStoreManager,
    /// 缓存的查询结果，供 detail 命令使用
    cached_events: Vec<RoutingEvent>,
    /// 命令行编辑器
    editor: DefaultEditor,
    /// 数据库路径
    db_path: String,
}

impl ReplApp {
    /// 创建新的 REPL 实例
    pub async fn new(db_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let config = StatisticsConfig {
            enabled: true,
            db_path: db_path.to_string(),
            retention_days: 30,
            write_buffer_size: 1000,
            aggregate_limit: 256,
        };
        let store = StatsStoreManager::new(&config).await?;
        let editor = DefaultEditor::new()?;

        Ok(Self {
            store,
            cached_events: Vec::new(),
            editor,
            db_path: db_path.to_string(),
        })
    }

    /// 运行 REPL 主循环
    pub async fn run(&mut self) -> ReadlineResult<()> {
        println!("LLM Gateway Statistics CLI");
        println!("Stats DB: {}\n", self.db_path);

        match self.store.count_events().await {
            Ok(count) => println!("Connected. Total events: {count}\n", count = count),
            Err(e) => println!("Warning: Could not count events: {e}\n", e = e),
        }

        loop {
            let readline = self.editor.readline("> ");

            match readline {
                Ok(line) => {
                    self.editor.add_history_entry(&line)?;

                    let command = Command::parse(&line);
                    let is_exit = matches!(command, Command::Exit);
                    self.handle_command(command).await;

                    if is_exit {
                        println!("Goodbye!");
                        break;
                    }
                }
                Err(rustyline::error::ReadlineError::Interrupted) => {
                    println!("Goodbye!");
                    break;
                }
                Err(rustyline::error::ReadlineError::Eof) => {
                    println!("Goodbye!");
                    break;
                }
                Err(err) => {
                    eprintln!("Error: {err}", err = err);
                    break;
                }
            }
        }

        Ok(())
    }

    /// 分发命令到对应处理器
    async fn handle_command(&mut self, command: Command) {
        match command {
            Command::Query { filter, format } => self.handle_query(filter, format).await,
            Command::Stats { query, .. } => self.handle_stats(query).await,
            Command::Models { sort, format } => self.handle_models(sort, format).await,
            Command::Backends { sort, format } => self.handle_backends(sort, format).await,
            Command::Recent { limit } => self.handle_recent(limit).await,
            Command::Detail { index } => self.handle_detail(index),
            Command::Help => self.show_help(),
            Command::Exit => {}
            Command::Unknown(cmd) => {
                if !cmd.is_empty() {
                    println!("Unknown command: {cmd}. Type 'help' for available commands.")
                }
            }
        }
    }

    /// 处理查询命令
    async fn handle_query(&mut self, filter: EventFilter, format: OutputFormat) {
        match self.store.query_events(filter.clone()).await {
            Ok(events) => {
                let count = events.len();
                let formatted = format_events(&events, format);
                self.cached_events = events;

                println!("Found {count} events:", count = count);
                println!("{formatted}", formatted = formatted);
            }
            Err(e) => {
                eprintln!("Error querying events: {e}", e = e);
            }
        }
    }

    /// 处理聚合统计命令
    ///
    /// 注意：model 和 backend 参数已在 query 中包含，预留用于未来扩展
    async fn handle_stats(&mut self, query: crate::query::AggQuery) {
        match self.store.get_aggregated_stats(query).await {
            Ok(result) => {
                println!("Aggregated statistics:");
                for stat in result.stats {
                    println!(
                        "  {model} / {backend}: {count} requests, {avg}ms avg",
                        model = stat.model,
                        backend = stat.backend,
                        count = stat.total_requests,
                        avg = stat.avg_duration_ms
                    );
                }
                println!(
                    "  Summary: {} at {}, remaining {}s ({})",
                    result.summary.stop_reason,
                    result.summary.window_start,
                    result.summary.window_size_seconds,
                    if result.summary.window_size_seconds == 0 {
                        "finished"
                    } else {
                        "too many data"
                    }
                );
            }
            Err(e) => {
                eprintln!("Error getting stats: {e}", e = e);
            }
        }
    }

    /// 处理列出模型命令
    ///
    /// 注意：sort 和 format 参数预留用于未来扩展
    async fn handle_models(&mut self, _sort: String, _format: OutputFormat) {
        let filter = EventFilter {
            limit: Some(1000),
            ..Default::default()
        };

        match self.store.query_events(filter).await {
            Ok(events) => {
                let mut model_counts: HashMap<String, usize> = HashMap::new();
                for event in events {
                    *model_counts.entry(event.model.clone()).or_insert(0) += 1;
                }

                let mut models: Vec<_> = model_counts.into_iter().collect();
                models.sort_by(|a, b| b.1.cmp(&a.1));

                println!("Available models:");
                for (model, count) in models {
                    println!(
                        "  {model:<20} ({count} events)",
                        model = model,
                        count = count
                    );
                }
            }
            Err(e) => {
                eprintln!("Error listing models: {e}", e = e);
            }
        }
    }

    /// 处理列出后端命令
    ///
    /// 注意：sort 和 format 参数预留用于未来扩展
    async fn handle_backends(&mut self, _sort: String, _format: OutputFormat) {
        let filter = EventFilter {
            limit: Some(1000),
            ..Default::default()
        };

        match self.store.query_events(filter).await {
            Ok(events) => {
                let mut backend_stats: HashMap<String, (usize, usize)> = HashMap::new();
                for event in events {
                    let entry = backend_stats.entry(event.backend.clone()).or_insert((0, 0));
                    entry.0 += 1;
                    if event.success {
                        entry.1 += 1;
                    }
                }

                let mut backends: Vec<_> = backend_stats.into_iter().collect();
                backends.sort_by(|a, b| b.1.0.cmp(&a.1.0));

                println!("Available backends:");
                for (backend, (total, success)) in backends {
                    let rate = if total > 0 {
                        (success as f64 / total as f64) * 100.0
                    } else {
                        0.0
                    };
                    println!(
                        "  {backend:<20} ({total} events, {rate:.1}% success)",
                        backend = backend,
                        total = total,
                        rate = rate
                    );
                }
            }
            Err(e) => {
                eprintln!("Error listing backends: {e}", e = e);
            }
        }
    }

    /// 处理最近事件命令
    async fn handle_recent(&mut self, limit: usize) {
        let filter = EventFilter {
            limit: Some(limit),
            ..Default::default()
        };

        match self.store.query_events(filter).await {
            Ok(events) => {
                let formatted = format_events(&events, OutputFormat::Table);
                self.cached_events = events;
                let count = self.cached_events.len();

                println!("Recent {count} events:", count = count);
                println!("{formatted}", formatted = formatted);
            }
            Err(e) => {
                eprintln!("Error getting recent events: {e}", e = e);
            }
        }
    }

    /// 处理查看事件详情命令
    fn handle_detail(&self, index: usize) {
        match self.cached_events.get(index) {
            Some(event) => {
                println!("Event #{idx} Details:", idx = index);
                println!("────────────────────────────────────────");

                let timestamp = Local
                    .timestamp_millis_opt(event.timestamp as i64)
                    .single()
                    .unwrap_or_default();

                println!(
                    "Timestamp:    {}",
                    timestamp.format("%Y-%m-%dT%H:%M:%S%.3f%:z")
                );
                println!("Model:        {model}", model = event.model);
                println!("Backend:      {backend}", backend = event.backend);
                println!("Duration:     {d}ms", d = event.duration_ms);
                println!(
                    "Success:      {}",
                    if event.success { "✓ Yes" } else { "✗ No" }
                );
                println!();
                println!("Request:");
                println!(
                    "  Client:     {}.{}.{}.{}:{port}",
                    (event.remote_addr >> 24) & 0xFF,
                    (event.remote_addr >> 16) & 0xFF,
                    (event.remote_addr >> 8) & 0xFF,
                    event.remote_addr & 0xFF,
                    port = event.remote_port
                );
                println!("  Method:     {method}", method = event.method);
                println!("  Path:       {path}", path = event.path);
                println!("  Input Port: {port}", port = event.input_port);
                println!(
                    "  Size:       {size} bytes",
                    size = event.request_size.unwrap_or(0)
                );
                println!();
                println!("Response:");
                println!(
                    "  Size:       {size} bytes",
                    size = event.response_size.unwrap_or(0)
                );
                println!();
                println!("Routing Path:");
                println!("  {path}", path = event.routing_path);
                println!();
                if let Some(ref error) = event.error_type {
                    println!("Error:        {error}", error = error);
                } else {
                    println!("Error:        (none)");
                }
            }
            None => {
                if self.cached_events.is_empty() {
                    println!("No events cached. Run 'query' or 'recent' first.");
                } else {
                    let len = self.cached_events.len();
                    println!(
                        "Invalid index. Cache contains {len} events (0-{last}).",
                        len = len,
                        last = len - 1
                    );
                }
            }
        }
    }

    /// 显示帮助信息
    fn show_help(&self) {
        println!("Available commands:");
        println!("  query     Query raw events with filters");
        println!("  stats     Show aggregated statistics");
        println!("  models    List all models");
        println!("  backends  List all backends");
        println!("  recent    Show recent events (shortcut)");
        println!("  detail    Show details of a cached query result");
        println!("  help      Show this help message");
        println!("  exit      Exit the CLI");
        println!();
        println!("Examples:");
        println!("  query --last 1h --model qwen-35b --limit 10");
        println!("  query --last 24h --format json");
        println!("  stats --model qwen-35b --last 24h");
        println!("  recent -n 20");
        println!("  detail 0");
    }
}
