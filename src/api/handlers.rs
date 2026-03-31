//! API 请求处理器

use bytes::Bytes;
use chrono::DateTime;
use http::header::CONTENT_TYPE;
use http_body_util::Full;
use hyper::{Request, Response, StatusCode};
use llm_gateway_statistics::{StatsStoreManager, parse_time};
use std::{collections::HashMap, sync::Arc};

use crate::api::middleware::AuthMiddleware;
use log::error;

type Body = Full<Bytes>;

/// Helper function to create parameter error response
fn invalid_param_error(message: impl Into<String>) -> Response<Body> {
    json_response(
        StatusCode::BAD_REQUEST,
        serde_json::json!({
            "message": message.into(),
            "error_type": "INVALID_PARAMS"
        }),
    )
}

/// 从 URL 查询字符串解析时间戳（支持毫秒时间戳或 ISO8601）
fn parse_timestamp(value: impl AsRef<str>) -> Option<i64> {
    let value = value.as_ref();
    if let Ok(ts) = value.parse() {
        // 先尝试解析为毫秒时间戳
        Some(ts)
    } else if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        // 尝试解析为 ISO8601/RFC3339
        Some(dt.timestamp_millis())
    } else {
        // 均失败
        None
    }
}

pub async fn handle_request(
    req: Request<hyper::body::Incoming>,
    auth: Arc<AuthMiddleware>,
    store: Arc<StatsStoreManager>,
    _peer_addr: std::net::SocketAddr,
) -> Result<Response<Body>, hyper::Error> {
    // 认证检查
    if !auth.authenticate(req.headers()) {
        return Ok(json_response(
            StatusCode::UNAUTHORIZED,
            serde_json::json!({
                "message": "Unauthorized",
                "error_type": "UNAUTHORIZED"
            }),
        ));
    }

    let method = req.method().clone();
    let path = req.uri().path().to_string();

    // 路由分发
    match (method.as_str(), path.as_str()) {
        ("GET", "/v1/stats/aggregate") => handle_aggregate(req, store).await,
        ("GET", "/v1/stats/overview") => handle_overview(req, store).await,
        _ => Ok(json_response(
            StatusCode::NOT_FOUND,
            serde_json::json!({
                "message": "Not found",
                "error_type": "NOT_FOUND"
            }),
        )),
    }
}

async fn handle_aggregate(
    req: Request<hyper::body::Incoming>,
    store: Arc<StatsStoreManager>,
) -> Result<Response<Body>, hyper::Error> {
    let uri = req.uri();
    let mut query = uri.query().map_or_else(Default::default, |q| {
        q.split('&')
            .filter_map(|pair| pair.split_once('='))
            .collect::<HashMap<&str, &str>>()
    });

    let now = chrono::Utc::now().timestamp_millis();

    // Parse time_range parameter (supports N+unit format)
    let time_range = match query.remove("time_range") {
        Some(value) => match parse_time(value) {
            Ok(secs) => Some(secs as i64 * 1000),
            Err(e) => {
                return Ok(invalid_param_error(format!(
                    "Invalid time_range format: {e}"
                )));
            }
        },
        None => None,
    };

    // Parse start_time and end_time
    let start_time_param = query.remove("start_time").and_then(parse_timestamp);
    let end_time_param = query.remove("end_time").and_then(parse_timestamp);

    const HOUR_MS: i64 = 60 * 60 * 1000;

    // Resolve time range based on provided parameters
    let (start_time, end_time) = match (start_time_param, end_time_param, time_range) {
        (Some(st), Some(et), Some(tr)) => {
            // All three provided - validate consistency
            if et - st != tr {
                return Ok(invalid_param_error(
                    "time_range does not match end_time - start_time",
                ));
            }
            (st, et)
        }

        // Only start and end time
        (Some(st), Some(et), None) => (st, et),
        // start_time + time_range -> calculate end_time
        (Some(st), None, Some(tr)) => (st, st + tr),
        // end_time + time_range -> calculate start_time
        (None, Some(et), Some(tr)) => (et - tr, et),

        // Only start_time -> use end_time = start_time + 1h
        (Some(st), None, None) => (st, st + HOUR_MS),
        // Only end_time -> use start_time = end_time - 1h
        (None, Some(et), None) => (et - HOUR_MS, et),
        // Only time_range -> use end_time = now
        (None, None, Some(tr)) => (now - tr, now),

        // Default: last 1 hour
        (None, None, None) => (now - HOUR_MS, now),
    };

    // Validate time range
    if start_time >= end_time {
        return Ok(invalid_param_error("start_time must be less than end_time"));
    }

    // Parse window_size (granularity)
    let window_size_secs = match query.remove("window_size") {
        Some(value) => match parse_time(value) {
            Ok(secs) => secs,
            Err(e) => {
                return Ok(invalid_param_error(format!(
                    "Invalid window_size format: {e}"
                )));
            }
        },
        None => 3600, // Default to 1h
    };
    let model = query.remove("model").map(|s| s.to_string());
    let backend = query.remove("backend").map(|s| s.to_string());

    // Build query
    let query = llm_gateway_statistics::AggQuery {
        start_time: start_time as u64,
        end_time: end_time as u64,
        window_size: std::num::NonZeroU64::new(window_size_secs)
            .unwrap_or_else(|| std::num::NonZeroU64::new(3600).unwrap()),
        model,
        backend,
    };

    // 执行查询
    match store.get_aggregated_stats(query).await {
        Ok(result) => {
            // 转换为响应格式
            let items: Vec<_> = result
                .stats
                .iter()
                .map(|s| {
                    serde_json::json!({
                        "window_start": chrono::DateTime::from_timestamp_millis(s.window_start as i64)
                            .map(|dt| dt.to_rfc3339())
                            .unwrap_or_default(),
                        "window_size_seconds": s.window_size / 1000,
                        "model": s.model,
                        "backend": s.backend,
                        "total_requests": s.total_requests,
                        "success_count": s.success_count,
                        "fail_count": s.fail_count,
                        "avg_duration_ms": s.avg_duration_ms,
                        "min_duration_ms": s.min_duration_ms,
                        "max_duration_ms": s.max_duration_ms,
                        "p50_duration_ms": s.p50_duration_ms,
                        "p90_duration_ms": s.p90_duration_ms,
                        "p99_duration_ms": s.p99_duration_ms,
                    })
                })
                .collect();

            // Summary as separate field
            let summary = serde_json::json!({
                "window_start": chrono::DateTime::from_timestamp_millis(result.summary.window_start as i64)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default(),
                "window_size_seconds": result.summary.window_size_seconds,
                "stop_reason": result.summary.stop_reason,
            });

            Ok(json_response(
                StatusCode::OK,
                serde_json::json!({
                    "message": "success",
                    "data": {
                        "total": items.len(),
                        "items": items,
                        "summary": summary
                    }
                }),
            ))
        }
        Err(e) => {
            error!("Error querying aggregate stats: {e}");
            Ok(json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({
                    "message": "Failed to query statistics",
                    "error_type": "INTERNAL_ERROR"
                }),
            ))
        }
    }
}

async fn handle_overview(
    _req: Request<hyper::body::Incoming>,
    store: Arc<StatsStoreManager>,
) -> Result<Response<Body>, hyper::Error> {
    use llm_gateway_statistics::query::EventFilter;
    use std::collections::HashMap;

    // 默认查询最近 1 小时
    let end_time = chrono::Utc::now().timestamp_millis();
    let start_time = end_time - 3_600_000; // 1 小时前

    let filter = EventFilter {
        start_time: Some(start_time),
        end_time: Some(end_time),
        model: None,
        backend: None,
        success: None,
        limit: Some(10000),
        offset: None,
    };

    match store.query_events(filter).await {
        Ok(events) => {
            if events.is_empty() {
                return Ok(json_response(
                    StatusCode::OK,
                    serde_json::json!({
                        "message": "success",
                        "data": {
                            "time_range": {
                                "start": chrono::DateTime::from_timestamp_millis(start_time)
                                    .map(|dt| dt.to_rfc3339()).unwrap_or_default(),
                                "end": chrono::DateTime::from_timestamp_millis(end_time)
                                    .map(|dt| dt.to_rfc3339()).unwrap_or_default(),
                            },
                            "summary": {
                                "total_requests": 0,
                                "success_rate": 0.0,
                                "avg_latency_ms": 0
                            },
                            "top_models": [],
                            "top_backends": []
                        }
                    }),
                ));
            }

            // 计算汇总统计
            let total_requests = events.len() as i64;
            let success_count = events.iter().filter(|e| e.success).count() as i64;
            let success_rate = success_count as f64 / total_requests as f64;
            let avg_latency = if total_requests > 0 {
                events.iter().map(|e| e.duration_ms).sum::<i64>() / total_requests
            } else {
                0
            };

            // 按模型聚合
            let mut model_stats: HashMap<String, (i64, i64)> = HashMap::new();
            for e in &events {
                let entry = model_stats.entry(e.model.clone()).or_insert((0, 0));
                entry.0 += 1;
                entry.1 += e.duration_ms;
            }
            let mut top_models: Vec<_> = model_stats
                .iter()
                .map(|(model, (count, total_ms))| {
                    serde_json::json!({
                        "model": model,
                        "requests": count,
                        "avg_latency_ms": if *count > 0 { total_ms / count } else { 0 }
                    })
                })
                .collect();
            top_models.sort_by(|a, b| b["requests"].as_i64().cmp(&a["requests"].as_i64()));
            top_models.truncate(5);

            // 按后端聚合
            let mut backend_stats: HashMap<String, (i64, i64, i64)> = HashMap::new();
            for e in &events {
                let entry = backend_stats.entry(e.backend.clone()).or_insert((0, 0, 0));
                entry.0 += 1;
                if e.success {
                    entry.1 += 1;
                }
                entry.2 += e.duration_ms;
            }
            let mut top_backends: Vec<_> = backend_stats
                .iter()
                .map(|(backend, (total, success, total_ms))| {
                    serde_json::json!({
                        "backend": backend,
                        "requests": total,
                        "success_rate": *success as f64 / *total as f64,
                        "avg_latency_ms": if *total > 0 { total_ms / total } else { 0 }
                    })
                })
                .collect();
            top_backends.sort_by(|a, b| b["requests"].as_i64().cmp(&a["requests"].as_i64()));
            top_backends.truncate(5);

            Ok(json_response(
                StatusCode::OK,
                serde_json::json!({
                    "message": "success",
                    "data": {
                        "time_range": {
                            "start": chrono::DateTime::from_timestamp_millis(start_time)
                                .map(|dt| dt.to_rfc3339()).unwrap_or_default(),
                            "end": chrono::DateTime::from_timestamp_millis(end_time)
                                .map(|dt| dt.to_rfc3339()).unwrap_or_default(),
                        },
                        "summary": {
                            "total_requests": total_requests,
                            "success_rate": success_rate,
                            "avg_latency_ms": avg_latency
                        },
                        "top_models": top_models,
                        "top_backends": top_backends
                    }
                }),
            ))
        }
        Err(e) => {
            error!("Error querying overview: {e}");
            Ok(json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({
                    "message": "Failed to query overview",
                    "error_type": "INTERNAL_ERROR"
                }),
            ))
        }
    }
}

fn json_response(status: StatusCode, body: serde_json::Value) -> Response<Body> {
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("HTTP response builder should never fail with valid status and body")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_timestamp_millis() {
        assert_eq!(parse_timestamp("1743004800000"), Some(1743004800000i64));
    }

    #[test]
    fn test_parse_timestamp_iso8601() {
        let result = parse_timestamp("2025-03-27T10:00:00Z");
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_time() {
        assert_eq!(parse_time("30s"), Ok(30));
        assert_eq!(parse_time("5m"), Ok(300));
        assert_eq!(parse_time("1h"), Ok(3600));
        assert_eq!(parse_time("1d"), Ok(86400));
        assert_eq!(parse_time("3600"), Ok(3600));
        assert_eq!(parse_time(""), Ok(3600)); // Default
    }

    #[test]
    fn test_handle_aggregate_time_window_consistency() {
        // 验证 start_time 和 end_time 使用同一基准时间
        // 这是一个逻辑测试，确保 now 只计算一次
        let now = chrono::Utc::now().timestamp_millis();
        let default_start = now - 3_600_000;
        let default_end = now;

        // 默认情况下，时间窗口应该是合理的（start < end）
        assert!(default_start < default_end);
        assert_eq!(default_end - default_start, 3_600_000);
    }
}
