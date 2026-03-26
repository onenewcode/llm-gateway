# Anthropic Messages API 协议

## 概述

Anthropic Messages API 是 Claude 系列模型的官方接口，支持多模态输入（文本、图像、文档）和工具调用。

## API 端点

```text
POST /v1/messages
```

## 请求格式

### 请求头

| Header            | 值                      | 必填 |
|:------------------|:------------------------|:----:|
| X-API-Key         | `{API_KEY}`             | ✅   |
| Anthropic-Version | `2023-06-01`            | ✅   |
| Content-Type      | `application/json`      | ✅   |
| Anthropic-Beta    | `beta-feature-name`     | ❌   |

### 请求体结构

```typescript
interface MessagesRequest {
  model: string;
  max_tokens: number;
  messages: Message[];
  system?: string | TextBlockParam[];
  temperature?: number;
  top_p?: number;
  top_k?: number;
  stop_sequences?: string[];
  stream?: boolean;
  tools?: ToolUnion[];
  tool_choice?: ToolChoice;
  thinking?: ThinkingConfig;
  metadata?: Metadata;
}
```

### 字段说明

| 字段           | 类型           | 必填 | 默认值  | 描述                     |
|:---------------|:---------------|:----:|:-------:|:-------------------------|
| model          | string         | ✅   | -       | 模型标识符               |
| max_tokens     | number         | ✅   | -       | 最大生成 token 数（≥ 1） |
| messages       | Message[]      | ✅   | -       | 对话历史消息数组         |
| system         | string\|array  | ❌   | -       | 系统提示                 |
| temperature    | number         | ❌   | `1.0`   | 采样温度（0.0-1.0）      |
| top_p          | number         | ❌   | `1.0`   | Nucleus 采样参数         |
| top_k          | number         | ❌   | -       | Top-K 采样（≥ 0）        |
| stop_sequences | string[]       | ❌   | -       | 自定义停止序列           |
| stream         | boolean        | ❌   | `false` | 是否启用 SSE 流式响应    |
| tools          | ToolUnion[]    | ❌   | -       | 可用工具列表             |
| tool_choice    | ToolChoice     | ❌   | `auto`  | 工具选择策略             |
| thinking       | ThinkingConfig | ❌   | -       | 扩展思考配置（beta）     |
| metadata       | object         | ❌   | -       | 请求元数据               |

### Message 对象

```typescript
interface Message {
  role: 'user' | 'assistant';
  content: string | ContentBlock[];
}
```

| 字段    | 类型          | 必填 | 描述     |
|:--------|:--------------|:----:|:---------|
| role    | string        | ✅   | 消息角色 |
| content | string\|array | ✅   | 消息内容 |

### ContentBlock 类型

```typescript
type ContentBlock =
  | TextBlockParam
  | ImageBlockParam
  | DocumentBlockParam
  | ToolResultBlockParam
  | ToolUseBlockParam
  | ThinkingBlockParam
  | SearchResultBlockParam;
```

#### TextBlockParam

```typescript
{
  type: "text";
  text: string;
  cache_control?: { type: "ephemeral"; ttl: "5m" | "1h" };
}
```

#### ImageBlockParam

```typescript
{
  type: "image";
  source: Base64ImageSource | URLImageSource;
}

type Base64ImageSource = {
  type: "base64";
  media_type: "image/jpeg" | "image/png" | "image/gif" | "image/webp";
  data: string;
};

type URLImageSource = {
  type: "url";
  url: string;
};
```

#### ToolUseBlockParam

```typescript
{
  type: "tool_use";
  id: string;
  name: string;
  input: Record<string, unknown>;
}
```

#### ToolResultBlockParam

```typescript
{
  type: "tool_result";
  tool_use_id: string;
  content?: string | ContentBlock[];
  is_error?: boolean;
}
```

### Tool 定义

```typescript
type ToolUnion = CustomTool | WebSearchTool;

interface CustomTool {
  type?: "custom";
  name: string;
  description?: string;
  input_schema: {
    type: "object";
    properties?: Record<string, unknown>;
    required?: string[];
  };
}

interface WebSearchTool {
  name: "web_search";
  type: "web_search_20250305";
  allowed_domains?: string[];
  blocked_domains?: string[];
  max_uses?: number;
  user_location?: {
    type: "approximate";
    city?: string;
    country?: string;
    region?: string;
    timezone?: string;
  };
}
```

### ToolChoice

```typescript
type ToolChoice =
  | { type: "auto"; disable_parallel_tool_use?: boolean }
  | { type: "any"; disable_parallel_tool_use?: boolean }
  | { type: "tool"; name: string; disable_parallel_tool_use?: boolean }
  | { type: "none" };
```

### ThinkingConfig

```typescript
type ThinkingConfig =
  | { type: "enabled"; budget_tokens: number }
  | { type: "disabled" };
```

### 请求示例

#### 非流式请求

```json
{
  "model": "claude-sonnet-4-5-20250929",
  "max_tokens": 1024,
  "messages": [
    {
      "role": "user",
      "content": "Hello, Claude!"
    }
  ]
}
```

#### 带系统提示的请求

```json
{
  "model": "claude-sonnet-4-5-20250929",
  "max_tokens": 1024,
  "system": "You are a helpful coding assistant.",
  "messages": [
    {
      "role": "user",
      "content": "Write a Python function to reverse a string."
    }
  ]
}
```

#### 流式请求

```json
{
  "model": "claude-sonnet-4-5-20250929",
  "max_tokens": 1024,
  "messages": [
    {
      "role": "user",
      "content": "Tell me a story."
    }
  ],
  "stream": true
}
```

## 响应格式（非流式）

### 响应体结构

```typescript
interface MessageResponse {
  id: string;
  type: "message";
  role: "assistant";
  model: string;
  content: ContentBlock[];
  stop_reason: string | null;
  stop_sequence: string | null;
  usage: Usage;
}
```

### 字段说明

| 字段          | 类型           | 描述                 |
|:--------------|:---------------|:---------------------|
| id            | string         | 消息唯一 ID          |
| type          | string         | 始终为 `"message"`   |
| role          | string         | 始终为 `"assistant"` |
| model         | string         | 使用的模型名称       |
| content       | ContentBlock[] | 响应内容块数组       |
| stop_reason   | string\|null   | 停止原因             |
| stop_sequence | string\|null   | 匹配的停止序列       |
| usage         | Usage          | Token 使用统计       |

### Usage 对象

```typescript
interface Usage {
  input_tokens: number;
  output_tokens: number;
  cache_creation_input_tokens?: number;
  cache_read_input_tokens?: number;
  cache_creation?: {
    ephemeral_5m_input_tokens: number;
    ephemeral_1h_input_tokens: number;
  };
  service_tier?: string;
}
```

| 字段                        | 描述                 |
|:----------------------------|:---------------------|
| input_tokens                | 输入 token 数        |
| output_tokens               | 输出 token 数        |
| cache_creation_input_tokens | 缓存创建的输入 token |
| cache_read_input_tokens     | 缓存读取的输入 token |
| service_tier                | 服务层级             |

### Stop Reason

| 值            | 描述               |
|:--------------|:-------------------|
| end_turn      | 自然结束           |
| max_tokens    | 达到 token 限制    |
| stop_sequence | 遇到停止序列       |
| tool_use      | 包含工具调用       |
| pause_turn    | 暂停（长运行工具） |
| refusal       | 安全拒绝           |

### 响应示例

```json
{
  "id": "msg_01ExampleID123",
  "type": "message",
  "role": "assistant",
  "model": "claude-sonnet-4-5-20250929",
  "content": [
    {
      "type": "text",
      "text": "Hello! How can I help you today?"
    }
  ],
  "stop_reason": "end_turn",
  "stop_sequence": null,
  "usage": {
    "input_tokens": 10,
    "output_tokens": 12,
    "service_tier": "standard"
  }
}
```

## 响应格式（流式 SSE）

### 响应头

| Header        | 值                  |
|:--------------|:--------------------|
| Content-Type  | `text/event-stream` |
| Cache-Control | `no-cache`          |
| Connection    | `keep-alive`        |

### SSE 事件类型

Anthropic 使用多种 SSE 事件类型：

| 事件类型            | 描述                   |
|:--------------------|:-----------------------|
| message_start       | 消息开始               |
| content_block_start | 内容块开始             |
| content_block_delta | 内容增量               |
| content_block_stop  | 内容块结束             |
| message_delta       | 消息增量（停止原因等） |
| message_stop        | 消息结束               |
| error               | 错误事件               |

### 事件详情

#### 1. message_start

```json
event: message_start
data: {
  "type": "message_start",
  "message": {
    "id": "msg_01ExampleID",
    "type": "message",
    "role": "assistant",
    "model": "claude-sonnet-4-5-20250929",
    "content": [],
    "stop_reason": null,
    "stop_sequence": null,
    "usage": {
      "input_tokens": 25,
      "output_tokens": 0
    }
  }
}
```

#### 2. content_block_start

```json
event: content_block_start
data: {
  "type": "content_block_start",
  "index": 0,
  "content_block": {
    "type": "text",
    "text": ""
  }
}
```

#### 3. content_block_delta

**文本增量：**

```json
event: content_block_delta
data: {
  "type": "content_block_delta",
  "index": 0,
  "delta": {
    "type": "text_delta",
    "text": "Hello, "
  }
}
```

**工具输入增量：**

```json
event: content_block_delta
data: {
  "type": "content_block_delta",
  "index": 1,
  "delta": {
    "type": "input_json_delta",
    "partial_json": "{\"location\": \"San Francisco\"}"
  }
}
```

#### 4. content_block_stop

```json
event: content_block_stop
data: {
  "type": "content_block_stop",
  "index": 0
}
```

#### 5. message_delta

```json
event: message_delta
data: {
  "type": "message_delta",
  "delta": {
    "stop_reason": "end_turn",
    "stop_sequence": null,
    "usage": {
      "output_tokens": 73
    }
  }
}
```

#### 6. message_stop

```json
event: message_stop
data: {
  "type": "message_stop"
}
```

#### 7. error

```json
event: error
data: {
  "type": "error",
  "error": {
    "type": "invalid_request_error",
    "message": "Your request is malformed."
  }
}
```

### 流式响应完整示例

```text
event: message_start
data: {"type":"message_start","message":{"id":"msg_01abc","type":"message","role":"assistant","model":"claude-sonnet-4-5-20250929","content":[],"stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":10,"output_tokens":0}}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"!"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn","usage":{"output_tokens":2}}}

event: message_stop
data: {"type":"message_stop"}
```

## 错误响应

### 错误格式

```json
{
  "type": "error",
  "error": {
    "type": "error_type",
    "message": "Error description"
  }
}
```

### 错误类型

| 类型                  | HTTP 状态码 | 描述         |
|:----------------------|:------------|:-------------|
| invalid_request_error | 400         | 请求格式错误 |
| authentication_error  | 401         | 认证失败     |
| permission_error      | 403         | 权限不足     |
| not_found_error       | 404         | 资源不存在   |
| rate_limit_error      | 429         | 速率限制     |
| api_error             | 500         | 服务器错误   |

## 协议识别特征

### Path 识别

- `/v1/messages` → Anthropic 协议

### Body 字段识别

- 包含 `messages` 数组 → 可能是 OpenAI 或 Anthropic
- 没有 `system` 角色，`system` 是独立字段 → Anthropic
- 包含 `max_tokens`（必填） → Anthropic
- `messages[].role` 只有 `user/assistant` → Anthropic

### 与 OpenAI 的区别

| 特征         | OpenAI                      | Anthropic            |
|:-------------|:----------------------------|:---------------------|
| 端点         | `/v1/chat/completions`      | `/v1/messages`       |
| 角色         | `system/user/assistant`     | `user/assistant`     |
| 系统提示     | `messages` 中的 system 角色 | 独立的 `system` 字段 |
| max_tokens   | 可选                        | **必填**             |
| 流式事件     | 简单 `delta.content`        | 多种事件类型         |
| 响应头       | `Authorization: Bearer`     | `X-API-Key`          |

## 实现注意事项

### 1. 非流式响应

- 直接返回完整 JSON 响应
- `Content-Type: application/json`

### 2. 流式响应

- 使用 SSE 格式，带事件类型
- `Content-Type: text/event-stream`
- 事件顺序：
  - `message_start`
  - `content_block_start`
  - `content_block_delta`（多次）
  - `content_block_stop`
  - `message_delta`
  - `message_stop`

### 3. 协议转换

**从 OpenAI 格式转换：**

- `messages` 中的 `system` 角色 → 独立的 `system` 字段
- 保持 `user/assistant` 角色不变

**从 Anthropic 格式转换：**

- 独立的 `system` 字段 → `messages` 中的 `system` 角色

### 4. 健康检查

- 成功响应 → 后端健康
- 连接错误/超时 → 后端不健康
- 4xx 错误 → 配置问题（不标记为不健康）
- 5xx 错误 → 后端问题（标记为不健康）

## 参考资料

### 官方文档

- [Anthropic API Documentation - Messages](https://docs.anthropic.com/claude/reference/messages)
- [Anthropic API Documentation - Streaming](https://docs.anthropic.com/claude/reference/messages-streaming)

### GitHub Issues

- [vllm: Support Anthropic API /v1/messages endpoint](https://github.com/vllm-project/vllm/issues/21313)
- [llama.cpp: Anthropic Messages API](https://huggingface.co/blog/ggml-org/anthropic-messages-api-in-llamacpp)
