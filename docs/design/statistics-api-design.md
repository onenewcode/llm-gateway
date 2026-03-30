# Statistics API 设计文档

## 1. 概述

本文档定义 LLM Gateway 统计查询功能的两种访问方式：

1. **Web API**: RESTful HTTP 接口，用于外部系统集成
2. **CLI 工具**: 对话式命令行界面，用于运维人员实时查询

**数据来源**：所有统计数据均来自 `stats.db` 数据库，CLI 运行时计算聚合。

## 2. 背景

### 2.1 现有架构

- `crates/statistics` 模块负责事件记录和统计聚合
- 数据存储在 SQLite (`stats.db`) 中
- 包含 `events` 表（原始事件）和 `aggregated_stats` 表（预计算聚合）

### 2.2 需求变更

- 启用 SQLite WAL 模式，支持多进程并发访问
- 新增 Web API 供外部查询聚合统计
- 新增 CLI 工具供运维查询原始事件和统计

## 3. 数据库配置

### 3.1 WAL 模式启用

```rust
// sqlite.rs - init_schema() 中添加
conn.execute("PRAGMA journal_mode=WAL;", [])?;
```

### 3.2 WAL 模式特性

| 特性      | 说明                             |
|-----------|----------------------------------|
| 读-读并发 | 支持多个进程同时读取             |
| 读-写并发 | 支持一个进程写入，多个进程读取   |
| 写-写并发 | 不支持，写入串行化               |
| 额外文件  | 生成 `.db-wal` 和 `.db-shm` 文件 |

### 3.3 访问模式

```plaintext
┌──────────────────────────────────────────────┐
│               stats.db (WAL Mode)            │
├──────────────────────────────────────────────┤
│  Gateway Process (写入)                      │
│  ┌───────────────────────────────────────┐   │
│  │ - record_event()                      │   │
│  │ - compute_aggregation()               │   │
│  │ - upsert_aggregated_stats()           │   │
│  └───────────────────────────────────────┘   │
├──────────────────────────────────────────────┤
│  Web API (读取)         CLI Tool (读取)      │
│  ┌───────────────────┐  ┌──────────────────┐ │
│  │ /v1/stats/aggrega │  │ query/stats      │ │
│  │ /v1/stats/overview│  │ models/backends  │ │
│  └───────────────────┘  └──────────────────┘ │
└──────────────────────────────────────────────┘
```

## 4. Web API 设计

### 4.1 基础信息

| 项目         | 值                                      |
|--------------|-----------------------------------------|
| Base URL     | `http://<gateway-host>:<admin-port>/v1` |
| Content-Type | `application/json`                      |
| 认证         | `Authorization: Bearer <token>`         |

**Admin Port 配置**（`config.toml`）：

```toml
[admin]
# 管理接口端口（默认随机分配）
# 如果未配置或配置为 0，则使用随机端口，启动时打印日志
port = 8080
# 认证 Token（为空则不启用认证）
auth-token = "your-secret-token"
```

**随机端口示例日志：**

```plaintext
[2025-03-27T10:23:45Z INFO] Admin API listening on http://0.0.0.0:49231/v1
```

### 4.2 端点列表

#### 4.2.1 聚合统计查询

```plaintext
GET /v1/stats/aggregate
```

**Query 参数：**

| 参数          | 类型   | 必填 | 说明                                                                      |
|:-------------:|:------:|:----:|---------------------------------------------------------------------------|
| `start_time`  | string | 是   | 开始时间（毫秒时间戳或 ISO8601 格式）                                     |
| `end_time`    | string | 是   | 结束时间（毫秒时间戳或 ISO8601 格式）                                     |
| `granularity` | string | 否   | 时间粒度：`5m`/`15m`/`1h`/`1d`，默认 `1h`                                 |
| `group_by`    | string | 否   | ~~分组维度：`model`/`backend`/`both`，默认 `both`~~ ⚠️ **当前版本未实现** |
| `model`       | string | 否   | 过滤特定模型                                                              |
| `backend`     | string | 否   | 过滤特定后端                                                              |

**时间格式示例：**

- 毫秒时间戳：`1743004800000`
- ISO8601：`2025-03-27T10:00:00Z`
- RFC3339：`2025-03-27T10:00:00+08:00`

**响应格式：**

```json
{
  "code": 200,
  "message": "success",
  "data": {
    "total": 3,
    "items": [
      {
        "window_start": "2025-03-27T10:00:00Z",
        "window_size_seconds": 3600,
        "model": "qwen-35b",
        "backend": "sglang-01",
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
    ]
  }
}
```

#### 4.2.2 实时概览

```plaintext
GET /v1/stats/overview
```

**Query 参数：** 无（返回最近 1 小时聚合）

**响应格式：**

```json
{
  "code": 200,
  "message": "success",
  "data": {
    "time_range": {
      "start": "2025-03-27T10:00:00Z",
      "end": "2025-03-27T11:00:00Z"
    },
    "summary": {
      "total_requests": 5234,
      "success_rate": 0.987,
      "avg_latency_ms": 245
    },
    "top_models": [
      { "model": "qwen-35b", "requests": 3000, "avg_latency_ms": 230 }
    ],
    "top_backends": [
      { "backend": "sglang-01", "requests": 3500, "success_rate": 0.99 }
    ]
  }
}
```

### 4.3 错误处理

| HTTP 状态码 | 错误码           | 说明                       |
|:-----------:|:----------------:|----------------------------|
| 400         | `INVALID_PARAMS` | 参数错误（如时间范围无效） |
| 401         | `UNAUTHORIZED`   | 认证失败                   |
| 403         | `FORBIDDEN`      | 权限不足                   |
| 500         | `INTERNAL_ERROR` | 内部错误                   |

**错误响应格式：**

```json
{
  "code": 400,
  "message": "Invalid time range: start_time must be less than end_time",
  "error_type": "INVALID_PARAMS"
}
```

## 5. CLI 接口设计

### 5.1 命令行工具

**命令：** `llm-stats`

**位置：** `crates/statistics/src/main.rs`

**用法：**

```bash
# 基本用法（使用默认数据库路径 ./stats.db）
llm-stats

# 指定数据库路径
llm-stats --db /var/lib/llm-gateway/stats.db

# 显示帮助
llm-stats --help
```

### 5.2 对话式交互

CLI 采用 REPL（Read-Eval-Print Loop）模式，启动后进入交互式命令行。

**状态管理**：

- 每次 `query` 命令的结果保存到会话缓存中
- 可通过 `detail <index>` 查看缓存中任意事件的详情
- 新查询会替换缓存，不会累积
- 会话退出后缓存丢失

**数据来源**：所有数据（events、models、backends）均来自 `stats.db` 数据库，运行时聚合计算。

```plaintext
$ llm-stats
LLM Gateway Statistics CLI
Stats DB: ./stats.db
Connected. Total events: 12,345

> help
Available commands:
  query     Query raw events with filters
  stats     Show aggregated statistics
  models    List all models
  backends  List all backends
  recent    Show recent events (shortcut)
  detail    Show details of a cached query result
  help      Show this help message
  exit      Exit the CLI

> query --last 1h --model qwen-35b --limit 10
Found 234 events (showing first 10):

Time                    Model        Backend       Duration  Success
─────────────────────────────────────────────────────────────────────
2025-03-27 10:23:45     qwen-35b     sglang-01        245ms  ✓
2025-03-27 10:23:42     qwen-35b     sglang-01        230ms  ✓
2025-03-27 10:23:38     qwen-35b     sglang-02        189ms  ✓
2025-03-27 10:23:35     qwen-35b     sglang-01       1200ms  ✗
2025-03-27 10:23:31     qwen-35b     sglang-01        267ms  ✓
...

> query --last 1h --model qwen-35b --format json
[
  {
    "timestamp": "2025-03-27T10:23:45.123Z",
    "model": "qwen-35b",
    "backend": "sglang-01",
    "duration_ms": 245,
    "success": true,
    "client": "192.168.1.100:54321"
  },
  ...
]

> stats --model qwen-35b --last 24h
Statistics for qwen-35b (last 24 hours):
────────────────────────────────────────
Total requests:     5,234
Success rate:       98.7%
Failed requests:    68

Latency:
  Average:  245ms
  Min:      120ms
  Max:      5,000ms
  P50:      230ms
  P90:      380ms
  P99:      1,200ms

Top backends:
  sglang-01:  3,500 requests (avg 240ms)
  sglang-02:  1,734 requests (avg 255ms)

> models
Available models:
  qwen-35b      (8,234 events)
  gpt-4         (2,156 events)
  llama-70b     (1,955 events)

> backends
Available backends:
  sglang-01     (5,500 events, 98.9% success)
  sglang-02     (3,234 events, 99.1% success)
  vllm-01       (2,345 events, 97.5% success)
  aliyun        (1,266 events, 99.8% success)

> recent
Recent 20 events:
...

> exit
Goodbye!
```

### 5.3 命令参考

#### 5.3.1 query - 查询原始事件

```plaintext
query [OPTIONS]

Options:
  --last <duration>     Time range (e.g., 5m, 15m, 1h, 24h, 7d)
  --start <timestamp>   Start time (ISO8601)
  --end <timestamp>     End time (ISO8601)
  --model <name>        Filter by model
  --backend <name>      Filter by backend
  --success <bool>      Filter by success (true/false)
  --limit <n>           Limit results (default: 100)
  --format <format>     Output format: table (default), json, csv
```

#### 5.3.2 stats - 聚合统计

```plaintext
stats [OPTIONS]

Options:
  --last <duration>     Time range (default: 1h)
  --start <timestamp>   Start time
  --end <timestamp>     End time
  --model <name>        Group by model
  --backend <name>      Group by backend
  --granularity <g>     Time window: 5m, 15m, 1h, 1d (default: 1h)
```

#### 5.3.3 models - 列出所有模型

```plaintext
models [OPTIONS]

Options:
  --sort <field>        Sort by: name, count, latency (default: count)
  --format <format>     Output format: table (default), json
```

#### 5.3.4 backends - 列出所有后端

```plaintext
backends [OPTIONS]

Options:
  --sort <field>        Sort by: name, count, success_rate (default: count)
  --format <format>     Output format: table (default), json
```

#### 5.3.5 recent - 快捷查看最近事件

```plaintext
recent [OPTIONS]

Options:
  -n <n>                Number of events (default: 20)
```

#### 5.3.6 detail - 查看缓存事件详情

```plaintext
detail <index>

显示最近一次 query 命令结果中指定索引的事件详情。

参数:
  <index>               事件索引（从 0 开始）

示例:
  > query --last 1h --limit 5
  > detail 0    # 查看第一个事件的详情
```

### 5.4 models 和 backends 数据来源

`models` 和 `backends` 命令的统计数据均来自 `events` 表的运行时聚合：

```sql
-- models 命令内部查询示例
SELECT model, COUNT(*) as event_count
FROM events
GROUP BY model
ORDER BY event_count DESC

-- backends 命令内部查询示例
SELECT backend, COUNT(*) as event_count,
       SUM(success) * 1.0 / COUNT(*) as success_rate
FROM events
GROUP BY backend
ORDER BY event_count DESC
```

所有聚合均为运行时计算，不依赖预计算表或配置文件。

## 6. 数据流

```plaintext
┌──────────────────────────────────────────────────────────────────────┐
│                         LLM Gateway                                  │
├──────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌──────────────┐      ┌──────────────┐      ┌──────────────────┐    │
│  │ Request Flow │─────▶│ Event Store  │─────▶│ stats.db (WAL)   │    │
│  └──────────────┘      └──────────────┘      └──────────────────┘    │
│                                │                       │             │
│                                │                       │             │
│                                ▼                       ▼             │
│                        ┌──────────────┐      ┌──────────────────┐    │
│                        │ Aggregator   │      │ Aggregated Stats │    │
│                        └──────────────┘      └──────────────────┘    │
│                                                                      │
├──────────────────────────────────────────────────────────────────────┤
│                              Query Interfaces                        │
│  ┌─────────────────────────────┐    ┌─────────────────────────────┐  │
│  │      Web API (serve)        │    │      CLI (llm-stats)        │  │
│  │  ┌───────────────────────┐  │    │  ┌───────────────────────┐  │  │
│  │  │ GET /v1/stats/aggrega │  │    │  │ query                 │  │  │
│  │  │ GET /v1/stats/overvie │  │    │  │ stats                 │  │  │
│  │  │                       │  │    │  │ models                │  │  │
│  │  └───────────────────────┘  │    │  │ backends              │  │  │
│  │        Read Only            │    │  │ recent                │  │  │
│  └─────────────────────────────┘    │  └───────────────────────┘  │  │
│                                     │        Read Only            │  │
│                                     └─────────────────────────────┘  │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

## 7. 实现计划

### 7.1 Phase 1: 数据库配置 ✅

- [x] 启用 SQLite WAL 模式（通过 `.gitignore` 配置 `-shm` 和 `-wal` 文件）
- [x] 验证多进程并发访问

### 7.2 Phase 2: Web API ✅

- [x] 在 `src/` 下创建 `api/` 目录
- [x] 实现 `AdminConfig` 配置解析（`llm_gateway_config::AdminConfig`）
- [x] 实现 `GET /v1/stats/aggregate` 端点（返回 ISO8601 时间格式）
- [x] 实现 `GET /v1/stats/overview` 端点
- [x] 添加 Bearer Token 认证中间件
- [x] Admin 端口支持随机分配（绑定端口 0）
- [x] 集成到 `main.rs`

**未实现的功能：**

- ⚠️ `group_by` 参数（设计支持 `model`/`backend`/`both`，当前仅按两者分组返回）

### 7.3 Phase 3: CLI 工具 ✅

- [x] 在 `crates/statistics/Cargo.toml` 添加 bin target
- [x] 添加依赖：`clap`、`chrono`、`humantime`、`rustyline`
- [x] 创建 `main.rs` 作为 CLI 入口
- [x] 创建 `cli/` 模块目录
- [x] 实现 REPL 交互循环
- [x] 实现 `query` 命令（支持过滤、分页、格式化）
- [x] 实现 `stats` 命令（运行时聚合计算）
- [x] 实现 `models` 和 `backends` 命令（从数据库聚合）
- [x] 实现 `recent` 和 `detail` 命令
- [x] 实现查询结果缓存

## 8. 依赖变更

### 8.1 crates/statistics/Cargo.toml

```toml
[[bin]]
name = "llm-stats"
path = "src/main.rs"

[dependencies]
# 现有依赖...

# CLI dependencies
clap = { version = "4.5", features = ["derive"] }
chrono = "0.4"
humantime = "2.1"
rustyline = "15"

[dev-dependencies]
# 现有 dev-dependencies...
```

## 9. 实现说明与差异

### 9.1 已实现功能（Web API）

当前实现已完成 Phase 1 和 Phase 2 的核心功能：

| 组件         | 文件路径                | 说明                                               |
|--------------|-------------------------|----------------------------------------------------|
| Admin Server | `src/api/admin.rs`      | HTTP 服务器，支持端口随机分配                      |
| Handlers     | `src/api/handlers.rs`   | `/v1/stats/aggregate` 和 `/v1/stats/overview` 端点 |
| Middleware   | `src/api/middleware.rs` | Bearer Token 认证中间件                            |
| Module       | `src/api/mod.rs`        | API 模块导出                                       |

### 9.2 设计与实现差异

| 设计项          | 实现状态   | 说明                                                                                        |
|-----------------|------------|---------------------------------------------------------------------------------------------|
| `group_by` 参数 | ⚠️ 未实现  | 设计支持按 `model`/`backend`/`both` 分组，当前版本仅支持按 `model` + `backend` 联合分组返回 |
| CLI 工具        | ✅ 已实现  | Phase 3 已完成，支持全部设计的命令                                                          |
| WAL 模式        | ✅ 已配置  | `.gitignore` 已添加 `*.db-shm` 和 `*.db-wal` 文件忽略                                       |

### 9.3 文件结构（实际实现）

```plaintext
crates/statistics/
├── Cargo.toml          # 已添加 bin target 和 CLI 依赖
├── src/
│   ├── lib.rs
│   ├── main.rs         # CLI 入口 (llm-stats)
│   ├── cli/            # CLI 模块目录 ✅
│   │   ├── mod.rs      # 模块导出
│   │   ├── repl.rs     # REPL 交互循环 ✅
│   │   ├── commands.rs # 命令解析与处理 ✅
│   │   └── formatter.rs # 输出格式化 (table/json/csv) ✅
│   ├── sqlite.rs       # WAL 模式已启用
│   ├── query.rs
│   ├── event.rs
│   ├── aggregator.rs
│   ├── store.rs
│   └── config.rs
```

### 9.4 代码审查修复记录

2026-03-30 代码审查后修复的问题：

1. **查询参数解析**：修复 `starts_with` 前缀匹配错误，改用 `split_once` 精确匹配
2. **时间窗口竞争条件**：修复 `start_time` 和 `end_time` 分别调用 `Utc::now()` 的问题
3. **日志输出**：统一使用 `tracing::error!` 替代 `eprintln!`
4. **除以零保护**：添加显式的除零检查
5. **代码清理**：删除未使用的 `_admin_config` 参数

## 10. 验收标准

### 10.1 Web API ✅

- [x] 可以通过 curl 成功查询聚合统计
- [x] 支持 Bearer Token 认证
- [x] 返回符合设计的 JSON 格式
- [x] 错误响应包含清晰的错误信息
- [x] 支持毫秒时间戳和 ISO8601 格式的时间参数

**遗留项：**

- [ ] `group_by` 参数支持（当前仅支持按 `model` + `backend` 联合分组）

### 10.2 CLI 工具 ✅

- [x] 可以独立运行 `llm-stats` 命令
- [x] 正确读取 `stats.db` 并显示事件列表
- [x] `query` 命令过滤器功能正常工作
- [x] `stats` 命令聚合统计正确
- [x] `models` 和 `backends` 命令显示数据库中的实际数据
- [x] `detail` 命令可查看缓存事件的详情
- [x] REPL 交互循环正常工作（help、exit 等）

**使用方法：**

```bash
# 编译并运行 CLI 工具
cargo run --bin llm-stats -- --db ./stats.db

# 或使用默认数据库路径
cargo run --bin llm-stats
```

### 10.3 并发测试

- [x] Gateway 进程持续写入时，Web API 可以正常读取
- [x] Gateway 进程持续写入时，CLI 可以正常读取
- [x] Web API 和 CLI 可以同时访问同一个数据库（通过 SQLite WAL 模式）
