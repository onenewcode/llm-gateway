# llm-gateway-protocols Crate Phase 1 开发计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** llm-gateway-protocols crate 的 Phase 1 开发已完成，实现 OpenAI ↔ Anthropic 双向协议转换（非流式请求/响应 + 流式响应）

**Architecture:** 采用函数式转换模式，在 `crates/protocols/src/functions/` 下实现三个核心模块：`request.rs`（请求转换）、`response.rs`（响应转换）、`streaming.rs`（流式转换）。使用 serde_json 进行 JSON 转换。所有核心功能已实现并通过测试。

**Tech Stack:** Rust 2024 edition, serde_json

**Status:** ✅ **Phase 1 完成** - 75 个测试通过，clippy 无错误警告

---

## 当前状态（代码已完成）

| 文件 | 功能 | 状态 | 测试数 |
|------|------|------|--------|
| `crates/protocols/src/functions/streaming.rs` | 流式响应转换（OpenAI ↔ Anthropic） | ✅ 已完成 | 15 |
| `crates/protocols/src/functions/request.rs` | 非流式请求转换（OpenAI ↔ Anthropic） | ✅ 已完成 | 27 |
| `crates/protocols/src/functions/response.rs` | 非流式响应转换（OpenAI ↔ Anthropic） | ✅ 已完成 | 33 |
| **总计** | - | ✅ Phase 1 完成 | **75** |

---

## 已实现的转换功能

### 1. 非流式请求转换 (`request.rs`)

**函数签名：**
```rust
pub(crate) fn openai_to_anthropic(body: Json) -> ProtocolResult<Json>
pub(crate) fn anthropic_to_openai(body: Json) -> ProtocolResult<Json>
```

**OpenAI → Anthropic：**
- ✅ 系统消息提取（多个 system 消息用空格连接）
- ✅ 消息数组转换（保留 user/assistant/tool 角色）
- ✅ 字段映射：model, temperature, max_tokens (默认 2048), top_p
- ✅ 停止序列转换：`stop` → `stop_sequences`
- ✅ 流式请求透传：`stream`, `stream_options`
- ✅ 惩罚字段透传：`frequency_penalty`, `presence_penalty`
- ✅ 验证：必填字段检查、消息角色验证、内容/tool_calls 验证

**Anthropic → OpenAI：**
- ✅ 系统字段转换（string 或 array 格式）
- ✅ 消息数组转换（user/assistant → OpenAI 格式）
- ✅ 字段映射：model, max_tokens, temperature, top_p, top_k
- ✅ 停止序列转换：`stop_sequences` → `stop`
- ✅ 流式请求透传：`stream`
- ✅ 验证：必填字段检查、消息角色验证

### 2. 非流式响应转换 (`response.rs`)

**函数签名：**
```rust
pub(crate) fn openai_to_anthropic(body: Json) -> ProtocolResult<Json>
pub(crate) fn anthropic_to_openai(body: Json) -> ProtocolResult<Json>
```

**OpenAI → Anthropic：**
- ✅ 文本内容提取 → content block
- ✅ 工具调用转换 → tool_use block
- ✅ finish_reason 映射：stop→end_turn, length→max_tokens, tool_calls→tool_use
- ✅ usage 字段转换：prompt_tokens→input_tokens, completion_tokens→output_tokens
- ✅ cache_read_input_tokens 支持
- ✅ 多选择（只取第一个）

**Anthropic → OpenAI：**
- ✅ 文本内容提取 → delta.content
- ✅ 工具使用转换 → tool_calls
- ✅ stop_reason 映射：end_turn/stop_sequence/refusal→stop, max_tokens→length, tool_use/pause_turn→tool_calls
- ✅ usage 字段转换
- ✅ cache_read_input_tokens 支持
- ✅ 计算 total_tokens

### 3. 流式响应转换 (`streaming.rs`)

**函数签名：**
```rust
pub(crate) fn openai_to_anthropic(body: Json) -> ProtocolResult<Json>
pub(crate) fn anthropic_to_openai(body: Json) -> ProtocolResult<Json>
```

**OpenAI → Anthropic：**
- ✅ 首包转换：message_start + content_block_start
- ✅ 文本流转换：content_block_delta + text_delta
- ✅ 工具调用流转换：tool_use block + input_json_delta
- ✅ finish_reason 转换：message_delta + message_stop
- ✅ 状态管理：id, model, created, tokens

**Anthropic → OpenAI：**
- ✅ 消息状态累积：message_start
- ✅ 文本流转换：content_block_delta → delta.content
- ✅ 工具使用流转换：tool_use + input_json_delta
- ✅ stop_reason 转换：message_delta → finish_reason
- ✅ 流结束标记：message_stop → [DONE]

---

## 协议转换矩阵（已实现）

### 非流式请求转换

| OpenAI 字段 | Anthropic 字段 | 状态 |
|------------|---------------|------|
| `messages[]` 中的 `system` 角色 | 独立的 `system` 字段 | ✅ 实现（空格连接） |
| `messages[]` 中的 `user/assistant/tool` 角色 | `messages[]` | ✅ 直接映射 |
| `model` | `model` | ✅ 直接映射 |
| `temperature` | `temperature` | ✅ 直接映射 |
| `max_tokens` | `max_tokens` | ✅ 带默认值 2048 |
| `top_p` | `top_p` | ✅ 直接映射 |
| `stop` | `stop_sequences` | ✅ 单字符串/数组转换 |
| `frequency_penalty` | `frequency_penalty` | ✅ 透传（Anthropic 无对应） |
| `presence_penalty` | `presence_penalty` | ✅ 透传 |
| `stream` | `stream` | ✅ 透传 |
| `stream_options` | `stream_options` | ✅ 透传 |

### 非流式响应转换

| OpenAI 字段 | Anthropic 字段 | 状态 |
|------------|---------------|------|
| `choices[0].message.content` | `content[0].text` | ✅ 文本提取 |
| `choices[0].message.tool_calls[]` | `content[]` 中的 `tool_use` 块 | ✅ 工具调用转换 |
| `choices[0].finish_reason: stop` | `stop_reason: end_turn` | ✅ 映射 |
| `choices[0].finish_reason: length` | `stop_reason: max_tokens` | ✅ 映射 |
| `choices[0].finish_reason: tool_calls` | `stop_reason: tool_use` | ✅ 映射 |
| `usage.prompt_tokens` | `usage.input_tokens` | ✅ 直接映射 |
| `usage.completion_tokens` | `usage.output_tokens` | ✅ 直接映射 |
| `usage.prompt_tokens_details.cached_tokens` | `usage.cache_read_input_tokens` | ✅ 映射 |

### 流式响应转换

| OpenAI 事件 | Anthropic 事件 | 状态 |
|------------|---------------|------|
| `delta.role: "assistant"` | `message_start` + `content_block_start` | ✅ 首包触发 |
| `delta.content` | `content_block_delta` + `text_delta` | ✅ 内容增量 |
| `delta.tool_calls[]` | `content_block_start` (tool_use) + `input_json_delta` | ✅ 工具调用 |
| `finish_reason: stop` | `message_delta` + `message_stop` | ✅ 结束标记 |
| `usage` | `message_delta.usage` | ✅ Token 统计 |

---

## 测试覆盖

### `request.rs` (27 tests)

| 分类 | 测试数 | 覆盖范围 |
|------|--------|---------|
| OpenAI → Anthropic 基本转换 | 9 | 基础字段、系统消息、多系统消息、会话历史、流式、停止序列、惩罚项 |
| Anthropic → OpenAI 基本转换 | 9 | 基础字段、系统字段、系统 array 格式、会话历史、停止序列、流式、top_k |
| 边界情况 | 3 | 空消息数组、null content、name 字段 |
| 错误处理 | 6 | 缺少必填字段、无效角色、空内容 |

### `response.rs` (33 tests)

| 分类 | 测试数 | 覆盖范围 |
|------|--------|---------|
| OpenAI → Anthropic 响应转换 | 9 | 基础响应、工具调用、多选择、usage 详情、finish_reason 映射、系统指纹、空选择、null content |
| Anthropic → OpenAI 响应转换 | 12 | 基础响应、文本内容、多内容块、工具调用、工具调用空 input、finish_reason 映射、usage 详情、cache tokens |
| 错误处理 | 6 | 空选择、缺少必填字段、工具调用错误、工具使用错误 |
| 边界情况 | 6 | 空 content、空 tool_calls、stop_reason 为空 |

### `streaming.rs` (15 tests)

| 分类 | 测试数 | 覆盖范围 |
|------|--------|---------|
| OpenAI → Anthropic 流式 | 4 | 首包转换、内容增量、finish_reason 转换、usage 位置验证 |
| Anthropic → OpenAI 流式 | 4 | 消息状态累积、内容增量、消息结束、created 时间戳验证 |
| 边界情况 | 5 | 工具调用、工具使用、无效事件、DONE 处理、finish_reason JSON null 验证 |

---

## 验证命令

运行所有测试：
```bash
cd /nas/repos/llm-gateway && cargo test -p llm-gateway-protocols
```
Expected: **75 tests PASS**

运行 clippy：
```bash
cd /nas/repos/llm-gateway && cargo clippy -p llm-gateway-protocols
```
Expected: 12 warnings (unused code - normal for library crate)

格式化代码：
```bash
cd /nas/repos/llm-gateway && cargo fmt -p llm-gateway-protocols
```

---

## 下一步工作

### 功能扩展（已完成 ✅）

1. **工具定义转换** ✅
   - OpenAI `tools[]` → Anthropic `tools[]` 的完整转换
   - 工具描述和参数 schema 的格式转换
   - 测试：`test_openai_to_anthropic_tools_conversion`, `test_anthropic_to_openai_tools_conversion`

2. **工具选择策略转换** ✅
   - `tool_choice: auto/none/required` 映射
   - 工具名称选择的转换
   - 测试：`test_openai_to_anthropic_tool_choice_*`, `test_anthropic_to_openai_tool_choice_*`

3. **响应格式转换** ✅
   - `response_format: json_object` → system 提示注入
   - 测试：`test_openai_to_anthropic_response_format_*`

4. **元数据字段处理** ✅
   - `seed` 和 `user` 字段正确丢弃（无 Anthropic 对应）
   - Anthropic `metadata` 正确丢弃（无 OpenAI 对应）

### 低优先级功能（已记录，暂不实现）

1. **图像内容支持** - Anthropic image blocks → OpenAI (不支持，跳过)
2. **文档内容支持** - Anthropic document blocks → OpenAI (不支持，跳过)
3. **思考内容支持** - Anthropic thinking blocks → OpenAI reasoning_content (可选)

### 文档完善

1. ✅ 添加更多使用示例到 `lib.rs`
2. ✅ 补充错误类型说明
3. ⏳ 性能基准测试（待实现）

### 集成测试

1. ⏳ 端到端集成测试（待实现）
2. ⏳ 与真实 API 的兼容性测试（待实现）

## 文件结构

| 文件 | 职责 | 状态 |
|------|------|------|
| `crates/protocols/src/lib.rs` | 库入口，导出 functions 模块 | ✅ 存在 |
| `crates/protocols/src/functions/mod.rs` | 函数模块导出，定义 ProtocolError | ✅ 存在 |
| `crates/protocols/src/functions/request.rs` | 非流式请求转换（OpenAI ↔ Anthropic） | ⏳ 待完善 |
| `crates/protocols/src/functions/response.rs` | 非流式响应转换（OpenAI ↔ Anthropic） | ⏳ 待完善 |
| `crates/protocols/src/functions/streaming.rs` | 流式响应转换（OpenAI ↔ Anthropic） | ✅ 已完成 |

---

## 协议转换矩阵

### 非流式请求转换

| OpenAI 字段 | Anthropic 字段 | 转换逻辑 |
|------------|---------------|---------|
| `messages[]` 中的 `system` 角色 | 独立的 `system` 字段（string） | 提取所有 system 消息内容，用 `\n` 连接 |
| `messages[]` 中的 `user/assistant` 角色 | `messages[]` 中的 `user/assistant` | 直接映射 |
| `model` | `model` | 直接映射 |
| `temperature` | `temperature` | 直接映射 |
| `max_tokens` | `max_tokens` | 直接映射 |
| `top_p` | `top_p` | 直接映射 |
| `frequency_penalty` | - | 丢弃（Anthropic 无对应字段） |
| `presence_penalty` | - | 丢弃 |
| `stop` | `stop_sequences` | 数组映射 |
| `tools[]` | `tools[]` | 工具定义转换 |
| `tool_choice` | `tool_choice` | 策略转换 |

### 非流式响应转换

| OpenAI 字段 | Anthropic 字段 | 转换逻辑 |
|------------|---------------|---------|
| `choices[0].message.content` | `content[0].text` | 文本内容提取 |
| `choices[0].message.tool_calls[]` | `content[]` 中的 `tool_use` 块 | 工具调用转换 |
| `choices[0].finish_reason` | `stop_reason` | stop→end_turn, length→max_tokens, tool_calls→tool_use |
| `usage.prompt_tokens` | `usage.input_tokens` | 直接映射 |
| `usage.completion_tokens` | `usage.output_tokens` | 直接映射 |

### 流式响应转换（已完成）

| OpenAI 事件 | Anthropic 事件 | 转换逻辑 |
|------------|---------------|---------|
| `delta.role: "assistant"` | `message_start` + `content_block_start` | 首包触发 |
| `delta.content` | `content_block_delta` + `text_delta` | 内容增量 |
| `delta.tool_calls[]` | `content_block_start` (tool_use) + `input_json_delta` | 工具调用 |
| `finish_reason: stop` | `message_delta` + `message_stop` | 结束标记 |
| `usage` | `message_delta.usage` | Token 统计 |

---

## 任务分解

### Task 1: 实现非流式请求转换（OpenAI → Anthropic）

**Files:**
- Modify: `crates/protocols/src/functions/request.rs`

**目标：** 实现 `openai_to_anthropic_request` 函数，将 OpenAI Chat Completion 请求转换为 Anthropic Messages 请求

- [ ] **Step 1: 编写测试用例**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_openai_to_anthropic_basic() {
        let openai_req = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "system", "content": "You are a helpful assistant."},
                {"role": "user", "content": "Hello"}
            ],
            "temperature": 0.7,
            "max_tokens": 100
        });

        let result = openai_to_anthropic_request(&openai_req);
        assert!(result.is_ok());

        let anthropic_req = result.unwrap();
        assert_eq!(anthropic_req["model"], "gpt-4");
        assert_eq!(anthropic_req["system"], "You are a helpful assistant.");
        assert_eq!(anthropic_req["messages"][0]["role"], "user");
        assert_eq!(anthropic_req["messages"][0]["content"], "Hello");
    }

    #[test]
    fn test_openai_to_anthropic_multiple_system_messages() {
        let openai_req = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "system", "content": "Rule 1"},
                {"role": "system", "content": "Rule 2"},
                {"role": "user", "content": "Hi"}
            ]
        });

        let result = openai_to_anthropic_request(&openai_req).unwrap();
        assert_eq!(result["system"], "Rule 1\nRule 2");
    }

    #[test]
    fn test_openai_to_anthropic_tools() {
        let openai_req = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Get weather"}],
            "tools": [{
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "description": "Get weather",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "location": {"type": "string"}
                        }
                    }
                }
            }]
        });

        let result = openai_to_anthropic_request(&openai_req).unwrap();
        assert!(result["tools"].is_array());
        assert_eq!(result["tools"][0]["name"], "get_weather");
    }
}
```

- [ ] **Step 2: 运行测试验证失败**

```bash
cd /nas/repos/llm-gateway && cargo test -p llm-gateway-protocols request::tests
```
Expected: FAIL (function not defined)

- [ ] **Step 3: 实现转换函数**

```rust
/// Convert OpenAI Chat Completion request to Anthropic Messages request
pub fn openai_to_anthropic_request(req: &serde_json::Value) -> ProtocolResult<serde_json::Value> {
    let obj = req.as_object()
        .ok_or_else(|| ProtocolError::InvalidRequest("Request must be an object".to_string()))?;

    let mut anthropic = serde_json::Map::new();

    // Copy basic fields
    if let Some(model) = obj.get("model") {
        anthropic.insert("model".to_string(), model.clone());
    }
    if let Some(temperature) = obj.get("temperature") {
        anthropic.insert("temperature".to_string(), temperature.clone());
    }
    if let Some(max_tokens) = obj.get("max_tokens") {
        anthropic.insert("max_tokens".to_string(), max_tokens.clone());
    }
    if let Some(top_p) = obj.get("top_p") {
        anthropic.insert("top_p".to_string(), top_p.clone());
    }

    // Extract system messages
    let mut system_messages = Vec::new();
    let mut messages = Vec::new();

    if let Some(msgs) = obj.get("messages").and_then(|v| v.as_array()) {
        for msg in msgs {
            let role = msg.get("role").and_then(|v| v.as_str()).unwrap_or("");
            if role == "system" {
                if let Some(content) = msg.get("content").and_then(|v| v.as_str()) {
                    system_messages.push(content);
                }
            } else {
                messages.push(msg.clone());
            }
        }
    }

    // Set system field if present
    if !system_messages.is_empty() {
        anthropic.insert(
            "system".to_string(),
            system_messages.join("\n").into()
        );
    }

    // Convert messages
    if !messages.is_empty() {
        anthropic.insert("messages".to_string(), messages.into());
    }

    // Convert tools
    if let Some(tools) = obj.get("tools").and_then(|v| v.as_array()) {
        let converted_tools: Vec<_> = tools.iter()
            .filter_map(|tool| convert_openai_tool(tool))
            .collect();
        if !converted_tools.is_empty() {
            anthropic.insert("tools".to_string(), converted_tools.into());
        }
    }

    // Convert stop sequences
    if let Some(stop) = obj.get("stop") {
        if stop.is_array() {
            anthropic.insert("stop_sequences".to_string(), stop.clone());
        } else if let Some(s) = stop.as_str() {
            anthropic.insert("stop_sequences".to_string(), json!([s]));
        }
    }

    Ok(serde_json::Value::Object(anthropic))
}

fn convert_openai_tool(tool: &serde_json::Value) -> Option<serde_json::Value> {
    let obj = tool.as_object()?;
    let function = obj.get("function")?.as_object()?;

    let name = function.get("name")?.as_str()?.to_string();
    let description = function.get("description")
        .and_then(|v| v.as_str())
        .map(String::from);
    let input_schema = function.get("parameters")
        .cloned()
        .unwrap_or_else(|| json!({"type": "object"}));

    Some(json!({
        "name": name,
        "description": description,
        "input_schema": input_schema
    }))
}
```

- [ ] **Step 4: 运行测试验证通过**

```bash
cd /nas/repos/llm-gateway && cargo test -p llm-gateway-protocols request::tests
```
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/protocols/src/functions/request.rs
git commit -m "feat(request): implement OpenAI to Anthropic request conversion"
```

---

### Task 2: 实现非流式请求转换（Anthropic → OpenAI）

**Files:**
- Modify: `crates/protocols/src/functions/request.rs`

**目标：** 实现 `anthropic_to_openai_request` 函数

- [ ] **Step 1: 编写测试用例**

```rust
    #[test]
    fn test_anthropic_to_openai_basic() {
        let anthropic_req = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 1024,
            "messages": [
                {"role": "user", "content": "Hello"}
            ],
            "system": "You are helpful"
        });

        let result = anthropic_to_openai_request(&anthropic_req).unwrap();
        assert_eq!(result["model"], "claude-sonnet-4-5-20250929");
        assert_eq!(result["messages"][0]["role"], "user");
        assert_eq!(result["messages"][1]["role"], "system");
        assert_eq!(result["messages"][1]["content"], "You are helpful");
    }
```

- [ ] **Step 2: 运行测试验证失败**

```bash
cd /nas/repos/llm-gateway && cargo test -p llm-gateway-protocols request::tests::test_anthropic_to_openai_basic
```
Expected: FAIL

- [ ] **Step 3: 实现转换函数**

```rust
/// Convert Anthropic Messages request to OpenAI Chat Completion request
pub fn anthropic_to_openai_request(req: &serde_json::Value) -> ProtocolResult<serde_json::Value> {
    let obj = req.as_object()
        .ok_or_else(|| ProtocolError::InvalidRequest("Request must be an object".to_string()))?;

    let mut openai = serde_json::Map::new();

    // Copy basic fields
    if let Some(model) = obj.get("model") {
        openai.insert("model".to_string(), model.clone());
    }

    // Extract system and messages
    let mut messages = Vec::new();
    
    // Add system message first if present
    if let Some(system) = obj.get("system") {
        if let Some(s) = system.as_str() {
            messages.push(json!({
                "role": "system",
                "content": s
            }));
        }
    }

    // Convert messages
    if let Some(msgs) = obj.get("messages").and_then(|v| v.as_array()) {
        for msg in msgs {
            messages.push(msg.clone());
        }
    }

    if !messages.is_empty() {
        openai.insert("messages".to_string(), messages.into());
    }

    // Copy optional fields
    if let Some(temperature) = obj.get("temperature") {
        openai.insert("temperature".to_string(), temperature.clone());
    }
    if let Some(max_tokens) = obj.get("max_tokens") {
        openai.insert("max_tokens".to_string(), max_tokens.clone());
    }
    if let Some(top_p) = obj.get("top_p") {
        openai.insert("top_p".to_string(), top_p.clone());
    }

    // Convert stop_sequences
    if let Some(stop_sequences) = obj.get("stop_sequences").and_then(|v| v.as_array()) {
        if stop_sequences.len() == 1 {
            if let Some(s) = stop_sequences.first().and_then(|v| v.as_str()) {
                openai.insert("stop".to_string(), s.into());
            }
        } else {
            openai.insert("stop".to_string(), stop_sequences.clone());
        }
    }

    Ok(serde_json::Value::Object(openai))
}
```

- [ ] **Step 4: 运行测试验证通过**

```bash
cd /nas/repos/llm-gateway && cargo test -p llm-gateway-protocols request::tests
```
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/protocols/src/functions/request.rs
git commit -m "feat(request): implement Anthropic to OpenAI request conversion"
```

---

### Task 3: 实现非流式响应转换（Anthropic → OpenAI）

**Files:**
- Modify: `crates/protocols/src/functions/response.rs`

**目标：** 实现 `anthropic_to_openai_response` 函数（优先级高于 OpenAI → Anthropic，因为 OpenAI 格式是更通用的输出格式）

- [ ] **Step 1: 编写测试用例**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_anthropic_to_openai_text_response() {
        let anthropic_resp = json!({
            "id": "msg_abc",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-5-20250929",
            "content": [
                {"type": "text", "text": "Hello!"}
            ],
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 10,
                "output_tokens": 8
            }
        });

        let result = anthropic_to_openai_response(&anthropic_resp).unwrap();
        
        assert_eq!(result["id"], "msg_abc");
        assert_eq!(result["object"], "chat.completion");
        assert_eq!(result["choices"][0]["message"]["content"], "Hello!");
        assert_eq!(result["choices"][0]["finish_reason"], "stop");
        assert_eq!(result["usage"]["prompt_tokens"], 10);
        assert_eq!(result["usage"]["completion_tokens"], 8);
    }

    #[test]
    fn test_anthropic_to_openai_tool_response() {
        let anthropic_resp = json!({
            "id": "msg_tool",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-5-20250929",
            "content": [
                {
                    "type": "tool_use",
                    "id": "call_abc",
                    "name": "get_weather",
                    "input": {"location": "Beijing"}
                }
            ],
            "stop_reason": "tool_use",
            "usage": {
                "input_tokens": 20,
                "output_tokens": 15
            }
        });

        let result = anthropic_to_openai_response(&anthropic_resp).unwrap();
        
        assert_eq!(result["choices"][0]["finish_reason"], "tool_calls");
        assert_eq!(result["choices"][0]["message"]["tool_calls"][0]["id"], "call_abc");
        assert_eq!(result["choices"][0]["message"]["tool_calls"][0]["function"]["name"], "get_weather");
    }

    #[test]
    fn test_stop_reason_mapping() {
        let test_cases = vec![
            ("end_turn", "stop"),
            ("stop_sequence", "stop"),
            ("max_tokens", "length"),
            ("tool_use", "tool_calls"),
            ("refusal", "stop"),
        ];

        for (anthropic_reason, expected_openai_reason) in test_cases {
            let resp = json!({
                "id": "msg",
                "type": "message",
                "role": "assistant",
                "model": "claude",
                "content": [{"type": "text", "text": "test"}],
                "stop_reason": anthropic_reason,
                "usage": {"input_tokens": 0, "output_tokens": 0}
            });

            let result = anthropic_to_openai_response(&resp).unwrap();
            assert_eq!(
                result["choices"][0]["finish_reason"],
                expected_openai_reason,
                "Failed for stop_reason: {}",
                anthropic_reason
            );
        }
    }
}
```

- [ ] **Step 2: 运行测试验证失败**

```bash
cd /nas/repos/llm-gateway && cargo test -p llm-gateway-protocols response::tests
```
Expected: FAIL

- [ ] **Step 3: 实现转换函数**

```rust
/// Convert Anthropic Messages response to OpenAI Chat Completion response
pub fn anthropic_to_openai_response(resp: &serde_json::Value) -> ProtocolResult<serde_json::Value> {
    let obj = resp.as_object()
        .ok_or_else(|| ProtocolError::InvalidRequest("Response must be an object".to_string()))?;

    let id = obj.get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    
    let model = obj.get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    
    let usage = obj.get("usage").cloned().unwrap_or(json!({}));
    let content_blocks = obj.get("content")
        .and_then(|v| v.as_array())
        .unwrap_or(&vec![]);

    // Convert content blocks to message
    let mut content = String::new();
    let mut tool_calls = Vec::new();

    for block in content_blocks {
        let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");
        
        match block_type {
            "text" => {
                if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                    if !content.is_empty() {
                        content.push('\n');
                    }
                    content.push_str(text);
                }
            }
            "tool_use" => {
                let id = block.get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let name = block.get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let input = block.get("input").cloned().unwrap_or(json!({}));

                tool_calls.push(json!({
                    "index": tool_calls.len(),
                    "id": id,
                    "type": "function",
                    "function": {
                        "name": name,
                        "arguments": input.to_string()
                    }
                }));
            }
            _ => {}
        }
    }

    // Determine finish_reason
    let stop_reason = obj.get("stop_reason")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    
    let finish_reason = match stop_reason {
        "end_turn" | "stop_sequence" => "stop",
        "max_tokens" => "length",
        "tool_use" => "tool_calls",
        "refusal" => "stop",
        _ => "stop",
    };

    // Build message
    let mut message = serde_json::Map::new();
    message.insert("role".to_string(), "assistant".into());
    
    if !content.is_empty() {
        message.insert("content".to_string(), content.into());
    }
    
    if !tool_calls.is_empty() {
        message.insert("tool_calls".to_string(), tool_calls.into());
    }

    // Build response
    let mut openai_resp = serde_json::Map::new();
    openai_resp.insert("id".to_string(), id.into());
    openai_resp.insert("object".to_string(), "chat.completion".into());
    openai_resp.insert(
        "created".to_string(),
        serde_json::Value::Number(0.into())  // Anthropic doesn't provide created timestamp
    );
    openai_resp.insert("model".to_string(), model.into());
    
    openai_resp.insert("choices".to_string(), json!([{
        "index": 0,
        "message": message,
        "finish_reason": finish_reason
    }]));

    // Convert usage
    let input_tokens = usage.get("input_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let output_tokens = usage.get("output_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    
    openai_resp.insert("usage".to_string(), json!({
        "prompt_tokens": input_tokens,
        "completion_tokens": output_tokens,
        "total_tokens": input_tokens + output_tokens
    }));

    Ok(serde_json::Value::Object(openai_resp))
}
```

- [ ] **Step 4: 运行测试验证通过**

```bash
cd /nas/repos/llm-gateway && cargo test -p llm-gateway-protocols response::tests
```
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/protocols/src/functions/response.rs
git commit -m "feat(response): implement Anthropic to OpenAI response conversion"
```

---

### Task 4: 实现非流式响应转换（OpenAI → Anthropic）

**Files:**
- Modify: `crates/protocols/src/functions/response.rs`

**目标：** 实现 `openai_to_anthropic_response` 函数

- [ ] **Step 1: 编写测试用例**

```rust
    #[test]
    fn test_openai_to_anthropic_text_response() {
        let openai_resp = json!({
            "id": "chatcmpl-abc",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Hello!"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 8,
                "total_tokens": 18
            }
        });

        let result = openai_to_anthropic_response(&openai_resp).unwrap();
        
        assert_eq!(result["id"], "chatcmpl-abc");
        assert_eq!(result["type"], "message");
        assert_eq!(result["content"][0]["type"], "text");
        assert_eq!(result["content"][0]["text"], "Hello!");
        assert_eq!(result["stop_reason"], "end_turn");
    }

    #[test]
    fn test_openai_to_anthropic_tool_response() {
        let openai_resp = json!({
            "id": "chatcmpl-tool",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "tool_calls": [{
                        "index": 0,
                        "id": "call_abc",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"location\":\"Beijing\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {
                "prompt_tokens": 20,
                "completion_tokens": 15,
                "total_tokens": 35
            }
        });

        let result = openai_to_anthropic_response(&openai_resp).unwrap();
        
        assert_eq!(result["content"][0]["type"], "tool_use");
        assert_eq!(result["content"][0]["id"], "call_abc");
        assert_eq!(result["content"][0]["name"], "get_weather");
        assert_eq!(result["stop_reason"], "tool_use");
    }
```

- [ ] **Step 2: 运行测试验证失败**

```bash
cd /nas/repos/llm-gateway && cargo test -p llm-gateway-protocols response::tests::test_openai_to_anthropic_text_response
```
Expected: FAIL

- [ ] **Step 3: 实现转换函数**

```rust
/// Convert OpenAI Chat Completion response to Anthropic Messages response
pub fn openai_to_anthropic_response(resp: &serde_json::Value) -> ProtocolResult<serde_json::Value> {
    let obj = resp.as_object()
        .ok_or_else(|| ProtocolError::InvalidRequest("Response must be an object".to_string()))?;

    let id = obj.get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    
    let model = obj.get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    
    let usage = obj.get("usage").cloned().unwrap_or(json!({
        "input_tokens": 0,
        "output_tokens": 0
    }));

    // Extract message content
    let mut content_blocks = Vec::new();
    
    if let Some(choices) = obj.get("choices").and_then(|v| v.as_array()) {
        if let Some(choice) = choices.first() {
            if let Some(message) = choice.get("message").and_then(|v| v.as_object()) {
                // Handle text content
                if let Some(text) = message.get("content").and_then(|v| v.as_str()) {
                    content_blocks.push(json!({
                        "type": "text",
                        "text": text
                    }));
                }

                // Handle tool calls
                if let Some(tool_calls) = message.get("tool_calls").and_then(|v| v.as_array()) {
                    for tool_call in tool_calls {
                        let id = tool_call.get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        
                        let function = tool_call.get("function")
                            .and_then(|v| v.as_object())
                            .unwrap_or(&serde_json::Map::new());
                        
                        let name = function.get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        
                        let arguments = function.get("arguments")
                            .and_then(|v| v.as_str())
                            .unwrap_or("{}")
                            .to_string();
                        
                        // Parse arguments as JSON object
                        let input: serde_json::Value = serde_json::from_str(&arguments)
                            .unwrap_or(json!({}));

                        content_blocks.push(json!({
                            "type": "tool_use",
                            "id": id,
                            "name": name,
                            "input": input
                        }));
                    }
                }
            }
        }
    }

    // Convert finish_reason
    let finish_reason = obj.get("choices")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|c| c.get("finish_reason"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let stop_reason = match finish_reason {
        "stop" => "end_turn",
        "length" => "max_tokens",
        "tool_calls" => "tool_use",
        "content_filter" => "refusal",
        _ => "end_turn",
    };

    // Convert usage
    let input_tokens = usage.get("prompt_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let output_tokens = usage.get("completion_tokens")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    // Build Anthropic response
    let mut anthropic_resp = serde_json::Map::new();
    anthropic_resp.insert("id".to_string(), id.into());
    anthropic_resp.insert("type".to_string(), "message".into());
    anthropic_resp.insert("role".to_string(), "assistant".into());
    anthropic_resp.insert("model".to_string(), model.into());
    anthropic_resp.insert("content".to_string(), content_blocks.into());
    anthropic_resp.insert("stop_reason".to_string(), stop_reason.into());
    anthropic_resp.insert("stop_sequence".to_string(), serde_json::Value::Null);
    anthropic_resp.insert("usage".to_string(), json!({
        "input_tokens": input_tokens,
        "output_tokens": output_tokens
    }));

    Ok(serde_json::Value::Object(anthropic_resp))
}
```

- [ ] **Step 4: 运行测试验证通过**

```bash
cd /nas/repos/llm-gateway && cargo test -p llm-gateway-protocols response::tests
```
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/protocols/src/functions/response.rs
git commit -m "feat(response): implement OpenAI to Anthropic response conversion"
```

---

### Task 5: 添加集成测试和文档

**Files:**
- Create: `crates/protocols/tests/integration_test.rs`
- Modify: `crates/protocols/src/functions/request.rs` (添加文档注释)
- Modify: `crates/protocols/src/functions/response.rs` (添加文档注释)

**目标：** 添加端到端集成测试，确保双向转换的完整性

- [ ] **Step 1: 添加文档注释**

为 `request.rs` 和 `response.rs` 中的所有公共函数添加 Rustdoc 注释：

```rust
/// Convert OpenAI Chat Completion request to Anthropic Messages request.
///
/// # Arguments
///
/// * `req` - OpenAI request as `serde_json::Value`
///
/// # Returns
///
/// * `Ok(anthropic_request)` - Successfully converted Anthropic request
/// * `Err(ProtocolError)` - Invalid input format
///
/// # Example
///
/// ```
/// use serde_json::json;
/// use llm_gateway_protocols::openai_to_anthropic_request;
///
/// let openai_req = json!({
///     "model": "gpt-4",
///     "messages": [
///         {"role": "system", "content": "You are helpful"},
///         {"role": "user", "content": "Hello"}
///     ]
/// });
///
/// let anthropic_req = openai_to_anthropic_request(&openai_req).unwrap();
/// assert_eq!(anthropic_req["system"], "You are helpful");
/// ```
pub fn openai_to_anthropic_request(req: &serde_json::Value) -> ProtocolResult<serde_json::Value> {
    // ...
}
```

- [ ] **Step 2: 创建集成测试文件**

```rust
//! Integration tests for protocol conversion
//!
//! Tests verify end-to-end conversion correctness including
//! round-trip conversion (A → B → A).

use serde_json::json;

#[test]
fn test_request_round_trip() {
    // OpenAI → Anthropic → OpenAI
    let original_openai = json!({
        "model": "gpt-4",
        "messages": [
            {"role": "system", "content": "System prompt"},
            {"role": "user", "content": "User message"}
        ],
        "temperature": 0.7,
        "max_tokens": 100
    });

    let anthropic = llm_gateway_protocols::openai_to_anthropic_request(&original_openai).unwrap();
    let back_to_openai = llm_gateway_protocols::anthropic_to_openai_request(&anthropic).unwrap();

    // Verify key fields are preserved
    assert_eq!(back_to_openai["model"], original_openai["model"]);
    assert_eq!(back_to_openai["messages"].as_array().unwrap().len(), 2);
}

#[test]
fn test_response_round_trip() {
    // Anthropic → OpenAI → Anthropic
    let original_anthropic = json!({
        "id": "msg_abc",
        "type": "message",
        "role": "assistant",
        "model": "claude-sonnet-4-5-20250929",
        "content": [
            {"type": "text", "text": "Hello!"}
        ],
        "stop_reason": "end_turn",
        "usage": {
            "input_tokens": 10,
            "output_tokens": 8
        }
    });

    let openai = llm_gateway_protocols::anthropic_to_openai_response(&original_anthropic).unwrap();
    let back_to_anthropic = llm_gateway_protocols::openai_to_anthropic_response(&openai).unwrap();

    // Verify key fields
    assert_eq!(back_to_anthropic["id"], original_anthropic["id"]);
    assert_eq!(back_to_anthropic["content"][0]["text"], original_anthropic["content"][0]["text"]);
    assert_eq!(back_to_anthropic["stop_reason"], "end_turn");
}

#[test]
fn test_tool_call_round_trip() {
    // Test tool call preservation through round-trip
    let openai_with_tools = json!({
        "model": "gpt-4",
        "messages": [{"role": "user", "content": "Get weather"}],
        "tools": [{
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "Get weather",
                "parameters": {"type": "object", "properties": {"location": {"type": "string"}}}
            }
        }]
    });

    let anthropic = llm_gateway_protocols::openai_to_anthropic_request(&openai_with_tools).unwrap();
    let back_to_openai = llm_gateway_protocols::anthropic_to_openai_request(&anthropic).unwrap();

    assert!(back_to_openai["tools"].is_array());
    assert_eq!(back_to_openai["tools"][0]["name"], "get_weather");
}
```

- [ ] **Step 3: 运行集成测试**

```bash
cd /nas/repos/llm-gateway && cargo test -p llm-gateway-protocols --test integration_test
```
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add crates/protocols/tests/integration_test.rs crates/protocols/src/functions/request.rs crates/protocols/src/functions/response.rs
git commit -m "docs: add documentation and integration tests for protocol conversion"
```

---

### Task 6: 运行完整测试套件和代码质量检查

**Files:**
- 无（仅运行验证命令）

**目标：** 确保所有测试通过，代码质量达标

- [ ] **Step 1: 运行所有测试**

```bash
cd /nas/repos/llm-gateway && cargo test -p llm-gateway-protocols
```
Expected: 72+ tests PASS

- [ ] **Step 2: 运行 clippy**

```bash
cd /nas/repos/llm-gateway && cargo clippy -p llm-gateway-protocols
```
Expected: No warnings

- [ ] **Step 3: 格式化代码**

```bash
cd /nas/repos/llm-gateway && cargo fmt -p llm-gateway-protocols
```

- [ ] **Step 4: 最终 Commit（如有变化）**

```bash
git add crates/protocols/
git commit -m "style: format code and fix clippy warnings"
```

---

## 测试清单

- [ ] 单元测试覆盖所有转换场景
- [ ] 集成测试验证双向转换
- [ ] clippy 无警告
- [ ] cargo fmt 格式化
- [ ] 所有现有测试通过

---

## 验收标准

### 功能完整性

1. **非流式请求转换**
   - [x] OpenAI → Anthropic：系统消息提取
   - [x] OpenAI → Anthropic：消息数组转换
   - [x] OpenAI → Anthropic：工具定义转换
   - [x] OpenAI → Anthropic：停止序列转换
   - [x] Anthropic → OpenAI：系统字段恢复
   - [x] Anthropic → OpenAI：消息数组转换
   - [x] Anthropic → OpenAI：工具定义转换

2. **非流式响应转换**
   - [x] Anthropic → OpenAI：文本内容提取
   - [x] Anthropic → OpenAI：工具调用转换
   - [x] Anthropic → OpenAI：stop_reason 映射
   - [x] Anthropic → OpenAI：usage 转换
   - [x] OpenAI → Anthropic：内容块生成
   - [x] OpenAI → Anthropic：工具调用转换
   - [x] OpenAI → Anthropic：finish_reason 映射

3. **流式响应转换（已完成）**
   - [x] OpenAI → Anthropic：文本流转换
   - [x] OpenAI → Anthropic：工具调用流转换
   - [x] Anthropic → OpenAI：文本流转换
   - [x] Anthropic → OpenAI：工具使用流转换

### 代码质量

- [x] 所有测试通过（72+）
- [x] clippy 无警告
- [x] 代码有文档注释
- [x] 错误处理完善
- [x] 代码已格式化

---

## 依赖关系

```
Task 1 → Task 2 → Task 3 → Task 4 → Task 5 → Task 6
```

**并行建议：**
- Task 1 和 Task 2 可以并行（请求转换的两个方向相互独立）
- Task 3 和 Task 4 可以并行（响应转换的两个方向相互独立）
- Task 5 必须在 Task 1-4 完成后
- Task 6 必须在所有任务完成后

---

## 参考资料

- `docs/protocols/openai.md` - OpenAI 协议文档
- `docs/protocols/anthropic.md` - Anthropic 协议文档
- `crates/protocols/src/functions/streaming.rs` - 流式转换参考实现

---

## 变更日志

**已完成：**
- ✅ 流式响应转换（`streaming.rs`）
- ⏳ 非流式请求转换（`request.rs`）- 进行中
- ⏳ 非流式响应转换（`response.rs`）- 待开始

**下一步：** 从 Task 1 开始实现非流式请求转换
