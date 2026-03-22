# OpenAI Chat Completion API 协议

## 概述

OpenAI Chat Completion API 是对话式 AI 应用的标准接口，被广泛兼容（vLLM, TGI, Ollama 等）。

## API 端点

```text
POST /v1/chat/completions
```

## 请求格式

### 请求头

| Header          | 值                  | 必填 |
|:----------------|:--------------------|:----:|
| Authorization   | `Bearer {API_KEY}`  | ✅   |
| Content-Type    | `application/json`  | ✅   |

### 请求体结构

```typescript
interface ChatCompletionRequest {
  model: string;
  messages: Message[];
  stream?: boolean;
  stream_options?: {
    include_usage?: boolean;
  };
  temperature?: number;
  max_tokens?: number;
  top_p?: number;
  frequency_penalty?: number;
  presence_penalty?: number;
  stop?: string | string[];
  tools?: Tool[];
  tool_choice?: 'auto' | 'none' | 'required' | ToolChoice;
  response_format?: { type: 'text' | 'json_object' };
  seed?: number;
  user?: string;
}
```

### 字段说明

| 字段                | 类型            | 必填 | 默认值   | 描述                          |
|:--------------------|:----------------|:----:|:--------:|:------------------------------|
| model               | string          | ✅   | -        | 模型标识符（如 `gpt-4`）      |
| messages            | Message[]       | ✅   | -        | 对话历史消息数组              |
| stream              | boolean         | ❌   | `false`  | 是否启用 SSE 流式响应         |
| stream_options      | object          | ❌   | -        | 流式选项配置                  |
| temperature         | number          | ❌   | `1.0`    | 采样温度（0.0-2.0）           |
| max_tokens          | number          | ❌   | `inf`    | 最大生成 token 数             |
| top_p               | number          | ❌   | `1.0`    | Nucleus 采样参数              |
| frequency_penalty   | number          | ❌   | `0.0`    | 频率惩罚（-2.0 到 2.0）       |
| presence_penalty    | number          | ❌   | `0.0`    | 存在惩罚（-2.0 到 2.0）       |
| stop                | string\|array   | ❌   | -        | 停止序列                      |
| tools               | Tool[]          | ❌   | -        | 可用工具列表                  |
| tool_choice         | string\|object  | ❌   | `auto`   | 工具选择策略                  |
| response_format     | object          | ❌   | -        | 响应格式（支持 JSON 模式）    |
| seed                | number          | ❌   | -        | 随机种子（可复现结果）        |
| user                | string          | ❌   | -        | 用户标识（用于监控）          |

### Message 对象

```typescript
interface Message {
  role: 'system' | 'user' | 'assistant' | 'tool';
  content: string | null;
  name?: string;
  tool_calls?: ToolCall[];
  tool_call_id?: string;
}
```

| 字段         | 类型          | 必填 | 描述                        |
|:-------------|:--------------|:----:|:----------------------------|
| role         | string        | ✅   | 消息角色                    |
| content      | string        | ❌   | 消息内容（可为 null）       |
| name         | string        | ❌   | 消息名称（可选）            |
| tool_calls   | ToolCall[]    | ❌   | 工具调用列表（assistant）   |
| tool_call_id | string        | ❌   | 工具调用 ID（tool 角色）    |

### 请求示例

#### 非流式请求

```json
{
  "model": "gpt-4",
  "messages": [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "Hello, how are you?"}
  ],
  "temperature": 0.7,
  "max_tokens": 100
}
```

#### 流式请求

```json
{
  "model": "gpt-4",
  "messages": [
    {"role": "user", "content": "Count from 1 to 10"}
  ],
  "stream": true,
  "stream_options": {
    "include_usage": true
  }
}
```

## 响应格式（非流式）

### 响应体结构

```typescript
interface ChatCompletionResponse {
  id: string;
  object: 'chat.completion';
  created: number;
  model: string;
  choices: Choice[];
  usage: Usage;
  system_fingerprint?: string;
}
```

### Choice 对象

```typescript
interface Choice {
  index: number;
  message: Message;
  finish_reason: 'stop' | 'length' | 'tool_calls' | 'content_filter' | null;
  logprobs?: object | null;
}
```

### Usage 对象

```typescript
interface Usage {
  prompt_tokens: number;
  completion_tokens: number;
  total_tokens: number;
  prompt_tokens_details?: {
    cached_tokens: number;
  };
  completion_tokens_details?: {
    reasoning_tokens: number;
    accepted_prediction_tokens: number;
    rejected_prediction_tokens: number;
  };
}
```

### 响应示例

```json
{
  "id": "chatcmpl-abc123",
  "object": "chat.completion",
  "created": 1234567890,
  "model": "gpt-4",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "Hello! How can I help you today?"
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 10,
    "completion_tokens": 8,
    "total_tokens": 18
  }
}
```

## 响应格式（流式 SSE）

### 响应头

| Header            | 值                  |
|:------------------|:--------------------|
| Content-Type      | `text/event-stream` |
| Cache-Control     | `no-cache`          |
| Connection        | `keep-alive`        |
| Transfer-Encoding | `chunked`           |

### SSE 消息格式

每个 SSE 消息格式：

```text
data: {JSON_PAYLOAD}\n\n
```

### Chunk 对象结构

```typescript
interface ChatCompletionChunk {
  id: string;
  object: 'chat.completion.chunk';
  created: number;
  model: string;
  choices: ChunkChoice[];
  system_fingerprint?: string;
  usage?: Usage;
}
```

### ChunkChoice 对象

```typescript
interface ChunkChoice {
  index: number;
  delta: Delta;
  finish_reason: 'stop' | 'length' | 'tool_calls' | 'content_filter' | null;
  logprobs?: object | null;
}
```

### Delta 对象

```typescript
interface Delta {
  role?: 'assistant';
  content?: string;
  tool_calls?: ToolCallDelta[];
  reasoning_content?: string;
}
```

### 流式响应示例

```text
data: {"id":"chatcmpl-abc","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}]}

data: {"id":"chatcmpl-abc","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}

data: {"id":"chatcmpl-abc","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{"content":", "},"finish_reason":null}]}

data: {"id":"chatcmpl-abc","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{"content":"how can I help you?"},"finish_reason":null}]}

data: {"id":"chatcmpl-abc","object":"chat.completion.chunk","created":1234567890,"model":"gpt-4","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":10,"completion_tokens":8,"total_tokens":18}}

data: [DONE]
```

### 流式事件序列

1. **首包** - 包含 `delta.role: "assistant"`
2. **内容包** - 包含 `delta.content: "partial text"`（多个）
3. **结束包** - 包含 `finish_reason` 和可选的 `usage`
4. **DONE** - `data: [DONE]` 标记流结束

## Finish Reason 说明

| 值             | 描述                                 |
|:---------------|:-------------------------------------|
| stop           | 自然结束（遇到停止序列或 EOS token） |
| length         | 达到 `max_tokens` 限制               |
| tool_calls     | 模型调用了工具                       |
| content_filter | 内容被过滤器拦截                     |
| null           | 流式响应中，表示仍在生成             |

## 错误响应

### 错误格式

```json
{
  "error": {
    "message": "Error message description",
    "type": "error_type",
    "param": null,
    "code": "error_code"
  }
}
```

### 常见错误码

| Code                      | HTTP 状态码 | 描述             |
|:--------------------------|:------------|:-----------------|
| invalid_api_key           | 401         | API Key 无效     |
| context_length_exceeded   | 400         | 输入超上下文长度 |
| rate_limit_exceeded       | 429         | 超过速率限制     |
| insufficient_quota        | 429         | 配额不足         |

## 协议识别特征

### Path 识别

- `/v1/chat/completions` → OpenAI 协议

### Body 字段识别

- 包含 `messages` 数组 → 可能是 OpenAI 或 Anthropic
- `messages[].role` 为 `system/user/assistant` → OpenAI
- 包含 `temperature`, `max_tokens` 字段 → OpenAI 风格

### 与 Anthropic 的区别

| 特征         | OpenAI                      | Anthropic              |
|:-------------|:----------------------------|:-----------------------|
| 端点         | `/v1/chat/completions`      | `/v1/messages`         |
| 角色         | `system/user/assistant`     | `user/assistant`       |
| 系统提示     | `messages` 中的 system 角色 | 独立的 `system` 字段   |
| 流式事件     | 简单 `delta.content`        | 多种事件类型           |

## 实现注意事项

### 1. 非流式响应

- 直接返回完整 JSON 响应
- `Content-Type: application/json`

### 2. 流式响应

- 使用 SSE 格式发送多个 chunk
- `Content-Type: text/event-stream`
- 每个 chunk 后跟两个换行符 `\n\n`
- 最后一个 chunk 后发送 `data: [DONE]\n\n`

### 3. 协议转换

- 作为 Gateway，需要支持 OpenAI 格式输入
- 转发到后端时保持 OpenAI 格式（如果后端兼容）
- 如果后端是 Anthropic，需要进行协议转换

### 4. 健康检查

- 成功响应 → 后端健康
- 连接错误/超时 → 后端不健康
- 4xx 错误 → 配置问题（不标记为不健康）
- 5xx 错误 → 后端问题（标记为不健康）

## 参考资料

### 官方文档

- [OpenAI API Documentation - Chat Completions](https://platform.openai.com/docs/api-reference/chat)
- [OpenAI API Documentation - Streaming Responses](https://platform.openai.com/docs/guides/streaming-responses)

### 技术文章

- [Streaming API Implementation Guide](https://crazyrouter.com/blog/streaming-api-implementation-guide) - Crazyrouter

### GitHub Issues

- [LocalAI: The chat completion chunk object](https://github.com/mudler/LocalAI/issues/2101)
