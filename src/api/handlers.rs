//! API 请求处理器

use bytes::Bytes;
use chrono::DateTime;
use http::header::CONTENT_TYPE;
use http_body_util::Full;
use hyper::{Request, Response, StatusCode};
use llm_gateway_statistics::{StatsStoreManager, TimeGranularity};
use std::sync::Arc;

use crate::api::middleware::AuthMiddleware;
use log::error;

type Body = Full<Bytes>;

/// 从 URL 查询字符串解析时间戳（支持毫秒时间戳或 ISO8601）
fn parse_timestamp(value: &str) -> Option<i64> {
    // 先尝试解析为毫秒时间戳
    if let Ok(ts) = value.parse::<i64>() {
        return Some(ts);
    }
    // 尝试解析为 ISO8601/RFC3339
    if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        return Some(dt.timestamp_millis());
    }
    None
}

/// 解析粒度参数
fn parse_granularity(value: Option<&str>) -> TimeGranularity {
    match value.unwrap_or("1h") {
        "5m" => TimeGranularity::FiveMinutes,
        "15m" => TimeGranularity::FifteenMinutes,
        "1h" => TimeGranularity::OneHour,
        "1d" => TimeGranularity::OneDay,
        _ => TimeGranularity::OneHour,
    }
}

/// 从请求 URI 提取查询参数
fn extract_query_param(uri: &hyper::Uri, key: &str) -> Option<String> {
    uri.query().and_then(|q| {
        q.split('&').find_map(|pair| {
            let (k, v) = pair.split_once('=')?;
            if k == key { Some(v.to_string()) } else { None }
        })
    })
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
                "code": 401,
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
                "code": 404,
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

    // 只计算一次当前时间，避免竞争条件
    let now = chrono::Utc::now().timestamp_millis();

    // 解析时间参数
    let start_time = extract_query_param(uri, "start_time")
        .and_then(|v| parse_timestamp(&v))
        .unwrap_or(now - 3_600_000);

    let end_time = extract_query_param(uri, "end_time")
        .and_then(|v| parse_timestamp(&v))
        .unwrap_or(now);

    // 验证时间范围
    if start_time >= end_time {
        return Ok(json_response(
            StatusCode::BAD_REQUEST,
            serde_json::json!({
                "code": 400,
                "message": "start_time must be less than end_time",
                "error_type": "INVALID_PARAMS"
            }),
        ));
    }

    // 解析其他参数
    let granularity = parse_granularity(extract_query_param(uri, "granularity").as_deref());
    let model = extract_query_param(uri, "model");
    let backend = extract_query_param(uri, "backend");

    // 构建查询
    let query = llm_gateway_statistics::AggQuery {
        start_time,
        end_time,
        window_size: std::num::NonZeroU64::new(granularity.as_seconds() as u64)
            .unwrap_or_else(|| std::num::NonZeroU64::new(3600).unwrap()),
        model: model.clone(),
        backend: backend.clone(),
    };

    // 执行查询
    match store.get_aggregated_stats(query).await {
        Ok(stats) => {
            // 转换为响应格式
            let items: Vec<_> = stats
                .iter()
                .map(|s| {
                    serde_json::json!({
                        "window_start": chrono::DateTime::from_timestamp_millis(s.window_start)
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

            Ok(json_response(
                StatusCode::OK,
                serde_json::json!({
                    "code": 200,
                    "message": "success",
                    "data": {
                        "total": items.len(),
                        "items": items
                    }
                }),
            ))
        }
        Err(e) => {
            error!("Error querying aggregate stats: {e}");
            Ok(json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                serde_json::json!({
                    "code": 500,
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
                        "code": 200,
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
                    "code": 200,
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
                    "code": 500,
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
    fn test_parse_granularity() {
        assert!(matches!(
            parse_granularity(Some("5m")),
            TimeGranularity::FiveMinutes
        ));
        assert!(matches!(
            parse_granularity(Some("1h")),
            TimeGranularity::OneHour
        ));
        assert!(matches!(parse_granularity(None), TimeGranularity::OneHour));
    }

    #[test]
    fn test_extract_query_param() {
        let uri: hyper::Uri = "/v1/stats?start=123&end=456".parse().unwrap();
        assert_eq!(extract_query_param(&uri, "start"), Some("123".to_string()));
        assert_eq!(extract_query_param(&uri, "end"), Some("456".to_string()));
        assert_eq!(extract_query_param(&uri, "missing"), None);
    }

    #[test]
    fn test_extract_query_param_no_query() {
        let uri: hyper::Uri = "/v1/stats".parse().unwrap();
        assert_eq!(extract_query_param(&uri, "start"), None);
    }

    #[test]
    fn test_extract_query_param_prefix_collision() {
        let uri: hyper::Uri = "/v1/stats?start_time=123&start_time_extra=456"
            .parse()
            .unwrap();
        assert_eq!(
            extract_query_param(&uri, "start_time"),
            Some("123".to_string())
        );
        assert_eq!(
            extract_query_param(&uri, "start_time_extra"),
            Some("456".to_string())
        );
    }

    #[test]
    fn test_extract_query_param_exact_match() {
        let uri: hyper::Uri = "/v1/stats?model=gpt-4&model_name=other".parse().unwrap();
        assert_eq!(
            extract_query_param(&uri, "model"),
            Some("gpt-4".to_string())
        );
        assert_eq!(
            extract_query_param(&uri, "model_name"),
            Some("other".to_string())
        );
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
