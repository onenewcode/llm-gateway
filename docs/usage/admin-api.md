# Admin API

LLM Gateway 管理 API，提供统计查询功能。

## 基础信息

| 项目         | 值                                      |
|--------------|-----------------------------------------|
| Base URL     | `http://<gateway-host>:<admin-port>/v1` |
| Content-Type | `application/json`                      |

## 配置

```toml
[admin]
port = 8080
auth-token = "your-secret-token"  # 可选，不设置则不启用认证
```

## 认证

如果配置了 `auth-token`，请求需要携带认证头：

```http
Authorization: Bearer <auth-token>
```

未配置时，所有请求无需认证。

## 端点

### GET /v1/stats/overview

获取最近 1 小时的统计概览。

**请求参数**：无

**请求示例**：

```bash
curl http://localhost:8080/v1/stats/overview
```

**响应示例**：

```json
{
  "code": 200,
  "message": "success",
  "data": {
    "time_range": {
      "start": "2026-03-30T02:00:00+00:00",
      "end": "2026-03-30T03:00:00+00:00"
    },
    "summary": {
      "total_requests": 5234,
      "success_rate": 0.987,
      "avg_latency_ms": 245
    },
    "top_models": [
      { "model": "qwen3.5-35b-a3b", "requests": 3000, "avg_latency_ms": 230 }
    ],
    "top_backends": [
      { "backend": "sglang-qwen3.5-35b-a3b", "requests": 3500, "success_rate": 0.99, "avg_latency_ms": 240 }
    ]
  }
}
```

### GET /v1/stats/aggregate

获取聚合统计数据。

**请求参数**：

| 参数           | 类型   | 必填 | 默认值   | 说明                                              |
|:--------------:|:------:|:----:|:--------:|---------------------------------------------------|
| `start_time`   | string | 否   | 1 小时前 | 毫秒时间戳或 ISO8601                              |
| `end_time`     | string | 否   | 现在     | 毫秒时间戳或 ISO8601                              |
| `time_range`   | string | 否   | -        | 时间范围：`5m`/`15m`/`1h`/`1d`，与 start/end 互斥 |
| `window_size`  | string | 否   | `1h`     | 聚合窗口大小：`5m`/`15m`/`1h`/`1d`                |
| `model`        | string | 否   | -        | 过滤特定模型                                      |
| `backend`      | string | 否   | -        | 过滤特定后端                                      |

时间格式示例：

- 毫秒时间戳：`1743004800000`
- ISO8601：`2026-03-30T10:00:00Z`
- 时间范围：`1h`（1 小时）、`30m`（30 分钟）、`2d`（2 天）

**参数组合规则**：

- `start_time` + `end_time`：指定具体时间范围
- `start_time` + `time_range`：从 start_time 开始的范围
- `end_time` + `time_range`：结束于 end_time 的范围
- 三者都提供时：验证 `time_range == end_time - start_time`

**请求示例**：

```bash
# 默认参数（最近 1 小时）
curl "http://localhost:8080/v1/stats/aggregate"

# 指定时间范围（毫秒时间戳）
curl "http://localhost:8080/v1/stats/aggregate?start_time=1743307200000&end_time=1743310800000"

# ISO8601 格式
curl "http://localhost:8080/v1/stats/aggregate?start_time=2026-03-30T10:00:00Z&end_time=2026-03-30T12:00:00Z"

# 使用 time_range（最近 2 小时）
curl "http://localhost:8080/v1/stats/aggregate?time_range=2h"

# 15 分钟窗口大小
curl "http://localhost:8080/v1/stats/aggregate?window_size=15m"

# 过滤特定模型
curl "http://localhost:8080/v1/stats/aggregate?model=qwen3.5-35b-a3b"

# 组合查询：最近 1 小时，15 分钟窗口，过滤特定模型
curl "http://localhost:8080/v1/stats/aggregate?time_range=1h&window_size=15m&model=qwen3.5-35b-a3b"
```

**响应示例**：

```json
{
  "code": 200,
  "message": "success",
  "data": {
    "total": 3,
    "items": [
      {
        "window_start": "2026-03-30T10:00:00+00:00",
        "window_size_seconds": 900,
        "model": "qwen3.5-35b-a3b",
        "backend": "sglang-qwen3.5-35b-a3b",
        "total_requests": 1523,
        "success_count": 1498,
        "fail_count": 25,
        "avg_duration_ms": 245,
        "min_duration_ms": 120,
        "max_duration_ms": 5000,
        "p50_duration_ms": 230,
        "p90_duration_ms": 380,
        "p99_duration_ms": 1200
      }
    ],
    "summary": {
      "window_start": "2026-03-30T12:00:00+00:00",
      "window_size_seconds": 0,
      "stop_reason": "finished"
    }
  }
}
```

**限额触发时的响应**：

```json
{
  "code": 200,
  "message": "success",
  "data": {
    "total": 256,
    "items": [
      // ... 最多 256 条数据
    ],
    "summary": {
      "window_start": "2026-03-30T11:30:00+00:00",
      "window_size_seconds": 1,
      "stop_reason": "too_many_data"
    }
  }
}
```

## 错误响应

### HTTP 400 - 参数错误

```json
{
  "code": 400,
  "message": "start_time must be less than end_time",
  "error_type": "INVALID_PARAMS"
}
```

### HTTP 401 - 认证失败

```json
{
  "code": 401,
  "message": "Unauthorized",
  "error_type": "UNAUTHORIZED"
}
```

### HTTP 404 - 路由不存在

```json
{
  "code": 404,
  "message": "Not found",
  "error_type": "NOT_FOUND"
}
```

### HTTP 500 - 内部错误

```json
{
  "code": 500,
  "message": "Failed to query statistics",
  "error_type": "INTERNAL_ERROR"
}
```

## TypeScript 类型定义

```typescript
// 通用响应包装
interface ApiResponse<T> {
  code: number;
  message: string;
  data: T;
}

// 时间范围
interface TimeRange {
  start: string;  // ISO8601 格式
  end: string;    // ISO8601 格式
}

// 聚合统计项
interface AggregateItem {
  window_start: string;           // ISO8601 格式
  window_size_seconds: number;    // 窗口大小（秒）
  model: string;
  backend: string;
  total_requests: number;
  success_count: number;
  fail_count: number;
  avg_duration_ms: number;
  min_duration_ms: number;
  max_duration_ms: number;
  p50_duration_ms: number | null;
  p90_duration_ms: number | null;
  p99_duration_ms: number | null;
}

// 聚合摘要（指示完成状态）
interface AggregateSummary {
  window_start: string;           // ISO8601 格式
  window_size_seconds: number;    // 剩余秒数（0 表示完成）
  stop_reason: "finished" | "too_many_data";
}

// 聚合统计响应
interface AggregateResponse {
  total: number;
  items: AggregateItem[];
  summary: AggregateSummary;      // 新增摘要字段
}

// 概览统计汇总
interface Summary {
  total_requests: number;
  success_rate: number;
  avg_latency_ms: number;
}

// Top N 模型
interface TopModel {
  model: string;
  requests: number;
  avg_latency_ms: number;
}

// Top N 后端
interface TopBackend {
  backend: string;
  requests: number;
  success_rate: number;
  avg_latency_ms: number;
}

// 概览统计响应
interface OverviewData {
  time_range: TimeRange;
  summary: Summary;
  top_models: TopModel[];
  top_backends: TopBackend[];
}

// 时间粒度（窗口大小）
// 支持任意正整数 + 单位格式：Ns（秒）、Nm/Nmin（分）、Nh（时）、Nd（天）
// 示例：5m、15m、1h、1d、30s、90min、2h
type WindowSize = string;

// 聚合查询参数
interface AggregateParams {
  start_time?: string | number;   // 毫秒时间戳或 ISO8601
  end_time?: string | number;     // 毫秒时间戳或 ISO8601
  time_range?: string;            // 时间范围：支持任意正整数 + 单位
  window_size?: WindowSize;       // 聚合窗口大小：支持任意正整数 + 单位
  model?: string;
  backend?: string;
}

// API 客户端
class AdminApiClient {
  constructor(baseUrl: string, token?: string);

  // 获取概览统计
  async getOverview(): Promise<ApiResponse<OverviewData>>;

  // 获取聚合统计
  async getAggregate(params?: AggregateParams): Promise<ApiResponse<AggregateResponse>>;

  // 请求封装
  private async request<T>(path: string, params?: Record<string, string>): Promise<T>;
}
```

## 使用示例

### 基础使用

```typescript
const client = new AdminApiClient("http://localhost:8080");

// 获取概览
const overview = await client.getOverview();
console.log(overview.data.summary);

// 获取聚合数据
const aggregate = await client.getAggregate({
  time_range: "1h",
  window_size: "15m",
  model: "qwen3.5-35b-a3b"
});
```

### 带认证

```typescript
const client = new AdminApiClient("http://localhost:8080", "your-secret-token");

const overview = await client.getOverview();
```

### 完整示例

```typescript
interface ApiResponse<T> {
  code: number;
  message: string;
  data: T;
}

interface OverviewData {
  time_range: { start: string; end: string };
  summary: { total_requests: number; success_rate: number; avg_latency_ms: number };
  top_models: Array<{ model: string; requests: number; avg_latency_ms: number }>;
  top_backends: Array<{ backend: string; requests: number; success_rate: number }>;
}

class AdminApiClient {
  constructor(
    private baseUrl: string,
    private token?: string
  ) {}

  private async request<T>(path: string, params?: Record<string, string>): Promise<T> {
    const url = new URL(path, this.baseUrl);
    if (params) {
      Object.entries(params).forEach(([k, v]) => url.searchParams.set(k, v));
    }

    const headers: HeadersInit = { "Content-Type": "application/json" };
    if (this.token) {
      headers["Authorization"] = `Bearer ${this.token}`;
    }

    const res = await fetch(url.toString(), { headers });
    if (!res.ok) {
      throw new Error(`HTTP ${res.status}: ${await res.text()}`);
    }
    return res.json();
  }

  async getOverview(): Promise<ApiResponse<OverviewData>> {
    return this.request("/v1/stats/overview");
  }

  async getAggregate(params?: Record<string, string>): Promise<ApiResponse<any>> {
    return this.request("/v1/stats/aggregate", params);
  }
}

// 使用
async function main() {
  const client = new AdminApiClient("http://localhost:8080");

  const overview = await client.getOverview();
  console.log(`Total requests: ${overview.data.summary.total_requests}`);
  console.log(`Success rate: ${(overview.data.summary.success_rate * 100).toFixed(1)}%`);

  const aggregate = await client.getAggregate({
    start_time: "2026-03-30T10:00:00Z",
    end_time: "2026-03-30T12:00:00Z",
    window_size: "15m",
    model: "qwen3.5-35b-a3b"
  });
  console.log(`Windows: ${aggregate.data.total}`);
}

main().catch(console.error);
```

## cURL 测试脚本

```bash
#!/bin/bash
BASE_URL="http://localhost:8080/v1"
TOKEN="${ADMIN_TOKEN:-}"  # 可通过环境变量设置 Token

# 带认证头（如果配置了 auth-token）
if [ -n "$TOKEN" ]; then
  AUTH_HEADER="-H Authorization: Bearer $TOKEN"
else
  AUTH_HEADER=""
fi

echo "=== Admin API 测试 ==="
echo

echo "1. 概览统计："
curl -s $AUTH_HEADER "$BASE_URL/stats/overview" | jq '.data.summary'

echo
echo "2. 聚合统计（默认 1 小时）："
curl -s $AUTH_HEADER "$BASE_URL/stats/aggregate" | jq '.data'

echo
echo "3. 聚合统计（15 分钟窗口）："
curl -s $AUTH_HEADER "$BASE_URL/stats/aggregate?window_size=15m" | jq '.data.total'

echo
echo "4. 使用 time_range（最近 2 小时）："
curl -s $AUTH_HEADER "$BASE_URL/stats/aggregate?time_range=2h" | jq '.data.total'

echo
echo "5. 指定时间范围："
curl -s $AUTH_HEADER "$BASE_URL/stats/aggregate?start_time=1743307200000&end_time=1743310800000" | jq '.data.total'

echo
echo "6. 过滤特定模型："
curl -s $AUTH_HEADER "$BASE_URL/stats/aggregate?model=qwen3.5-35b-a3b" | jq '.data.total'

echo
echo "7. 组合查询（1 小时范围，15 分钟窗口）："
curl -s $AUTH_HEADER "$BASE_URL/stats/aggregate?time_range=1h&window_size=15m&model=qwen3.5-35b-a3b" | jq '.data'

echo
echo "8. 查看响应中的 summary 字段："
curl -s $AUTH_HEADER "$BASE_URL/stats/aggregate?window_size=5m" | jq '.data.summary'

echo
echo "9. 错误测试（无效时间范围）："
curl -s $AUTH_HEADER "$BASE_URL/stats/aggregate?start_time=9999999999999&end_time=0" | jq
```
