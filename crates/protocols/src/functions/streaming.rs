use super::{ProtocolError, ProtocolResult};
use serde_json::{Value as Json, json};

/// SSE line type for streaming responses
#[derive(Debug, Clone)]
pub enum SseLine {
    /// Event type line: `event: {type}`
    Event(String),
    /// Data line: `data: {json}`
    Data(Json),
    /// Done marker: `data: [DONE]`
    Done,
}

/// Streaming collector trait for protocol conversion
pub trait StreamingCollector {
    /// Process a streaming line and return converted lines (if any)
    fn insert(&mut self, line: SseLine) -> ProtocolResult<Option<Vec<SseLine>>>;
}

/// OpenAI to Anthropic streaming converter
#[derive(Default)]
pub(crate) struct OpenaiToAnthropic {
    id: Option<String>,
    model: Option<String>,
    created: Option<u64>,
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    initialized: bool,
}

impl StreamingCollector for OpenaiToAnthropic {
    fn insert(&mut self, line: SseLine) -> ProtocolResult<Option<Vec<SseLine>>> {
        let SseLine::Data(chunk) = line else {
            return Ok(None);
        };

        let mut ans = Vec::new();
        let obj = chunk
            .as_object()
            .ok_or_else(|| ProtocolError::InvalidRequest("Chunk must be an object".to_string()))?;

        // Initialize on first chunk
        if !self.initialized {
            if self.id.is_none()
                && let Some(id) = obj.get("id").and_then(|v| v.as_str())
            {
                self.id = Some(id.to_string());
            }
            if self.model.is_none()
                && let Some(model) = obj.get("model").and_then(|v| v.as_str())
            {
                self.model = Some(model.to_string());
            }
            if self.created.is_none()
                && let Some(created) = obj.get("created").and_then(|v| v.as_u64())
            {
                self.created = Some(created);
            }

            // Generate message_start event
            ans.push(SseLine::Event("message_start".to_string()));
            ans.push(SseLine::Data(self.create_message_start()));

            // Generate content_block_start event
            ans.push(SseLine::Event("content_block_start".to_string()));
            ans.push(SseLine::Data(self.create_content_block_start(0)));

            self.initialized = true;
        }

        // Process choices
        if let Some(choices) = obj.get("choices").and_then(|v| v.as_array())
            && let Some(choice) = choices.first()
        {
            // Process delta
            if let Some(delta) = choice.get("delta") {
                // Handle content delta
                if let Some(content) = delta.get("content").and_then(|v| v.as_str())
                    && !content.is_empty()
                {
                    ans.push(SseLine::Event("content_block_delta".to_string()));
                    ans.push(SseLine::Data(self.create_content_block_delta(0, content)));
                }

                // Handle tool calls
                if let Some(tool_calls) = delta.get("tool_calls").and_then(|v| v.as_array()) {
                    for (idx, tool_call) in tool_calls.iter().enumerate() {
                        if let Some(function) = tool_call.get("function") {
                            let tool_id = Self::extract_str(function, "id", "");
                            let name = Self::extract_str(function, "name", "");
                            let arguments = Self::extract_str(function, "arguments", "{}");

                            // Generate tool_use content_block_start
                            ans.push(SseLine::Event("content_block_start".to_string()));
                            ans.push(SseLine::Data(self.create_tool_use_block_start(
                                1 + idx,
                                &tool_id,
                                &name,
                            )));

                            // Generate tool_use input delta
                            ans.push(SseLine::Event("content_block_delta".to_string()));
                            ans.push(SseLine::Data(
                                self.create_input_json_delta(1 + idx, &arguments),
                            ));
                        }
                    }
                }
            }

            // Handle finish_reason
            if let Some(finish_reason) = choice.get("finish_reason").and_then(|v| v.as_str())
                && finish_reason != "null"
                && !finish_reason.is_empty()
            {
                // Generate content_block_stop
                ans.push(SseLine::Event("content_block_stop".to_string()));
                ans.push(SseLine::Data(json!({
                    "type": "content_block_stop",
                    "index": 0
                })));

                // Generate message_delta
                ans.push(SseLine::Event("message_delta".to_string()));
                ans.push(SseLine::Data(self.create_message_delta(finish_reason)));

                // Generate message_stop
                ans.push(SseLine::Event("message_stop".to_string()));
                ans.push(SseLine::Data(json!({"type": "message_stop"})));
            }
        }

        // Process usage
        if let Some(usage) = obj.get("usage")
            && let Some(usage_obj) = usage.as_object()
        {
            if let Some(prompt_tokens) = usage_obj.get("prompt_tokens").and_then(|v| v.as_u64()) {
                self.input_tokens = Some(prompt_tokens);
            }
            if let Some(completion_tokens) =
                usage_obj.get("completion_tokens").and_then(|v| v.as_u64())
            {
                self.output_tokens = Some(completion_tokens);
            }
        }

        Ok(Some(ans))
    }
}

impl OpenaiToAnthropic {
    /// Extract a string field from JSON with default value
    fn extract_str(obj: &Json, key: &str, default: &str) -> String {
        obj.get(key)
            .and_then(|v| v.as_str())
            .unwrap_or(default)
            .into()
    }

    fn create_message_start(&self) -> Json {
        json!({
            "type": "message_start",
            "message": {
                "id": self.id.as_deref().unwrap_or(""),
                "type": "message",
                "role": "assistant",
                "model": self.model.as_deref().unwrap_or(""),
                "content": [],
                "stop_reason": null,
                "stop_sequence": null,
                "usage": {
                    "input_tokens": 0,
                    "output_tokens": 0
                }
            }
        })
    }

    fn create_content_block_start(&self, index: usize) -> Json {
        json!({
            "type": "content_block_start",
            "index": index,
            "content_block": {
                "type": "text",
                "text": ""
            }
        })
    }

    fn create_content_block_delta(&self, index: usize, text: &str) -> Json {
        json!({
            "type": "content_block_delta",
            "index": index,
            "delta": {
                "type": "text_delta",
                "text": text
            }
        })
    }

    fn create_tool_use_block_start(&self, index: usize, tool_id: &str, name: &str) -> Json {
        json!({
            "type": "content_block_start",
            "index": index,
            "content_block": {
                "type": "tool_use",
                "id": tool_id,
                "name": name,
                "input": {}
            }
        })
    }

    fn create_input_json_delta(&self, index: usize, partial_json: &str) -> Json {
        json!({
            "type": "content_block_delta",
            "index": index,
            "delta": {
                "type": "input_json_delta",
                "partial_json": partial_json
            }
        })
    }

    fn create_message_delta(&mut self, finish_reason: &str) -> Json {
        // Convert finish_reason
        let stop_reason = match finish_reason {
            "stop" => "end_turn",
            "length" => "max_tokens",
            "tool_calls" => "tool_use",
            "content_filter" => "refusal",
            _ => "end_turn",
        };

        json!({
            "type": "message_delta",
            "delta": {
                "stop_reason": stop_reason,
                "stop_sequence": null
            },
            "usage": {
                "output_tokens": self.output_tokens.unwrap_or_default()
            }
        })
    }
}

/// Anthropic to OpenAI streaming converter
#[derive(Default)]
pub(crate) struct AnthropicToOpenai {
    id: Option<String>,
    model: Option<String>,
    created: Option<u64>,
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    current_block_type: Option<String>,
    tool_id: Option<String>,
    tool_name: Option<String>,
    tool_input: String,
}

impl StreamingCollector for AnthropicToOpenai {
    fn insert(&mut self, line: SseLine) -> ProtocolResult<Option<Vec<SseLine>>> {
        let SseLine::Data(event) = line else {
            return Ok(None);
        };

        let obj = event.as_object().ok_or_else(|| {
            ProtocolError::InvalidStreamEvent("Event must be an object".to_string())
        })?;

        let event_type = obj
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::InvalidStreamEvent("Event missing type".to_string()))?;

        match event_type {
            "message_start" => self.handle_message_start(obj),
            "content_block_start" => self.handle_content_block_start(obj),
            "content_block_delta" => self.handle_content_block_delta(obj),
            "content_block_stop" => self.handle_content_block_stop(obj),
            "message_delta" => self.handle_message_delta(obj),
            "message_stop" => self.handle_message_stop(),
            _ => Err(ProtocolError::InvalidStreamEvent(format!(
                "Unknown event type: {event_type}"
            ))),
        }
    }
}

impl AnthropicToOpenai {
    fn handle_message_start(
        &mut self,
        obj: &serde_json::Map<String, Json>,
    ) -> ProtocolResult<Option<Vec<SseLine>>> {
        if let Some(message) = obj.get("message").and_then(|v| v.as_object()) {
            if let Some(id) = message.get("id").and_then(|v| v.as_str()) {
                self.id = Some(id.to_string());
            }
            if let Some(model) = message.get("model").and_then(|v| v.as_str()) {
                self.model = Some(model.to_string());
            }
            if let Some(usage) = message.get("usage").and_then(|v| v.as_object())
                && let Some(input_tokens) = usage.get("input_tokens").and_then(|v| v.as_u64())
            {
                self.input_tokens = Some(input_tokens);
            }
        }
        // No output for message_start
        Ok(None)
    }

    fn handle_content_block_start(
        &mut self,
        obj: &serde_json::Map<String, Json>,
    ) -> ProtocolResult<Option<Vec<SseLine>>> {
        if let Some(content_block) = obj.get("content_block").and_then(|v| v.as_object()) {
            let block_type = content_block
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("text")
                .to_string();

            self.current_block_type = Some(block_type.clone());

            if block_type == "tool_use" {
                self.tool_id = content_block
                    .get("id")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                self.tool_name = content_block
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                self.tool_input = String::new();
            }
        }
        // No output for content_block_start
        Ok(None)
    }

    fn handle_content_block_delta(
        &mut self,
        obj: &serde_json::Map<String, Json>,
    ) -> ProtocolResult<Option<Vec<SseLine>>> {
        if let Some(delta) = obj.get("delta").and_then(|v| v.as_object()) {
            let delta_type = delta.get("type").and_then(|v| v.as_str()).unwrap_or("");

            match delta_type {
                "text_delta" => {
                    if let Some(text) = delta.get("text").and_then(|v| v.as_str()) {
                        // Generate OpenAI chunk with delta.content
                        return Ok(Some(vec![SseLine::Data(self.create_chunk(
                            None,
                            Some(text),
                            None,
                        ))]));
                    }
                }
                "input_json_delta" => {
                    if let Some(partial_json) = delta.get("partial_json").and_then(|v| v.as_str()) {
                        // Accumulate tool input
                        self.tool_input.push_str(partial_json);
                    }
                    // No output for input_json_delta
                }
                _ => {}
            }
        }
        Ok(None)
    }

    fn handle_content_block_stop(
        &mut self,
        _obj: &serde_json::Map<String, Json>,
    ) -> ProtocolResult<Option<Vec<SseLine>>> {
        // If this was a tool_use block, generate tool_calls chunk
        if self.current_block_type.as_deref() == Some("tool_use") {
            let tool_id = self.tool_id.take().unwrap_or_default();
            let tool_name = self.tool_name.take().unwrap_or_default();
            let tool_input = std::mem::take(&mut self.tool_input);

            let tool_calls_chunk = self.create_chunk(
                None,
                None,
                Some((tool_id.as_str(), tool_name.as_str(), tool_input.as_str())),
            );
            return Ok(Some(vec![SseLine::Data(tool_calls_chunk)]));
        }
        self.current_block_type = None;
        Ok(None)
    }

    fn handle_message_delta(
        &mut self,
        obj: &serde_json::Map<String, Json>,
    ) -> ProtocolResult<Option<Vec<SseLine>>> {
        // Extract stop_reason
        let stop_reason = obj
            .get("delta")
            .and_then(|v| v.as_object())
            .and_then(|d| d.get("stop_reason"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Convert stop_reason
        let finish_reason = match stop_reason {
            "end_turn" | "stop_sequence" => "stop",
            "max_tokens" => "length",
            "tool_use" => "tool_calls",
            "pause_turn" => "tool_calls",
            "refusal" => "stop",
            _ => "stop",
        };

        // Extract usage
        if let Some(usage) = obj.get("usage").and_then(|v| v.as_object())
            && let Some(output_tokens) = usage.get("output_tokens").and_then(|v| v.as_u64())
        {
            self.output_tokens = Some(output_tokens);
        }

        // Generate OpenAI chunk with finish_reason and usage
        Ok(Some(vec![SseLine::Data(self.create_chunk(
            Some(finish_reason),
            None,
            None,
        ))]))
    }

    fn handle_message_stop(&mut self) -> ProtocolResult<Option<Vec<SseLine>>> {
        // Generate [DONE] marker
        Ok(Some(vec![SseLine::Done]))
    }

    fn create_chunk(
        &self,
        finish_reason: Option<&str>,
        content: Option<&str>,
        tool_call: Option<(&str, &str, &str)>,
    ) -> Json {
        let mut delta = json!({});

        if let Some(content) = content {
            delta["content"] = json!(content);
        }

        if let Some((tool_id, tool_name, tool_input)) = tool_call {
            delta["tool_calls"] = json!([{
                "index": 0,
                "id": tool_id,
                "type": "function",
                "function": {
                    "name": tool_name,
                    "arguments": tool_input
                }
            }]);
        }

        let mut choice = json!({
            "index": 0,
            "delta": delta,
            "finish_reason": finish_reason.unwrap_or("null")
        });

        let mut result = json!({
            "id": self.id.as_deref().unwrap_or(""),
            "object": "chat.completion.chunk",
            "created": self.created.unwrap_or_default(),
            "model": self.model.as_deref().unwrap_or(""),
            "choices": [choice]
        });

        // Add usage if we have it and this is the final chunk
        if finish_reason.is_some() {
            result["usage"] = json!({
                "prompt_tokens": self.input_tokens.unwrap_or_default(),
                "completion_tokens": self.output_tokens.unwrap_or_default(),
                "total_tokens": self.input_tokens.unwrap_or_default() + self.output_tokens.unwrap_or_default()
            });
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ============================================================================
    // OpenAI → Anthropic 转换测试
    // ============================================================================

    #[test]
    fn test_openai_message_start_conversion() {
        let mut converter = OpenaiToAnthropic::default();
        let chunk = json!({
            "id": "chatcmpl-abc",
            "object": "chat.completion.chunk",
            "created": 1234567890,
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "delta": { "role": "assistant" },
                "finish_reason": null
            }]
        });

        let events = converter.insert(SseLine::Data(chunk)).unwrap();

        // Should generate message_start and content_block_start
        assert!(events.is_some());
        let events = events.unwrap();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, SseLine::Event(e) if e == "message_start"))
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, SseLine::Event(e) if e == "content_block_start"))
        );
    }

    #[test]
    fn test_openai_content_delta_conversion() {
        let mut converter = OpenaiToAnthropic::default();

        // First establish state
        let _ = converter.insert(SseLine::Data(json!({
            "id": "chatcmpl-abc",
            "model": "gpt-4",
            "choices": [{ "delta": { "role": "assistant" } }]
        })));

        // Then send content
        let chunk = json!({
            "choices": [{
                "delta": { "content": "Hello" },
                "finish_reason": null
            }]
        });

        let events = converter.insert(SseLine::Data(chunk)).unwrap();

        // Should generate content_block_delta with text_delta
        assert!(events.is_some());
        let events = events.unwrap();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, SseLine::Event(e) if e == "content_block_delta"))
        );
    }

    #[test]
    fn test_openai_finish_reason_conversion() {
        let mut converter = OpenaiToAnthropic::default();

        // Establish state
        let _ = converter.insert(SseLine::Data(json!({
            "choices": [{ "delta": { "role": "assistant" } }]
        })));

        // Send finish
        let chunk = json!({
            "choices": [{
                "delta": {},
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 8,
                "total_tokens": 18
            }
        });

        let events = converter.insert(SseLine::Data(chunk)).unwrap();

        // Should generate message_delta with stop_reason: end_turn
        assert!(events.is_some());
        let events = events.unwrap();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, SseLine::Event(e) if e == "message_delta"))
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, SseLine::Event(e) if e == "message_stop"))
        );
    }

    // ============================================================================
    // Anthropic → OpenAI 转换测试
    // ============================================================================

    #[test]
    fn test_anthropic_message_start_accumulation() {
        let mut converter = AnthropicToOpenai::default();

        // message_start should accumulate state but not produce output
        let event = json!({
            "type": "message_start",
            "message": {
                "id": "msg_abc",
                "type": "message",
                "role": "assistant",
                "model": "claude-sonnet-4-5-20250929",
                "usage": {
                    "input_tokens": 10,
                    "output_tokens": 0
                }
            }
        });

        let chunk = converter.insert(SseLine::Data(event)).unwrap();

        // Should return None (no output yet)
        assert!(chunk.is_none());
    }

    #[test]
    fn test_anthropic_content_delta_conversion() {
        let mut converter = AnthropicToOpenai::default();

        // Establish state
        let _ = converter.insert(SseLine::Data(json!({
            "type": "message_start",
            "message": {
                "id": "msg_abc",
                "model": "claude-sonnet-4-5-20250929",
                "usage": { "input_tokens": 10, "output_tokens": 0 }
            }
        })));

        // Send content delta
        let event = json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {
                "type": "text_delta",
                "text": "Hello"
            }
        });

        let chunk = converter.insert(SseLine::Data(event)).unwrap();

        // Should generate OpenAI chunk with delta.content
        assert!(chunk.is_some());
        let lines = chunk.unwrap();
        assert!(lines.iter().any(
            |e| matches!(e, SseLine::Data(d) if d["choices"][0]["delta"]["content"] == "Hello")
        ));
    }

    #[test]
    fn test_anthropic_message_delta_finish() {
        let mut converter = AnthropicToOpenai::default();

        // Establish state
        let _ = converter.insert(SseLine::Data(json!({
            "type": "message_start",
            "message": {
                "id": "msg_abc",
                "model": "claude-sonnet-4-5-20250929",
                "usage": { "input_tokens": 10, "output_tokens": 0 }
            }
        })));

        // Send message_delta
        let event = json!({
            "type": "message_delta",
            "delta": {
                "stop_reason": "end_turn"
            },
            "usage": {
                "output_tokens": 8
            }
        });

        let chunk = converter.insert(SseLine::Data(event)).unwrap();

        // Should generate OpenAI chunk with finish_reason: stop
        assert!(chunk.is_some());
        let lines = chunk.unwrap();
        assert!(
            lines.iter().any(
                |e| matches!(e, SseLine::Data(d) if d["choices"][0]["finish_reason"] == "stop")
            )
        );
    }

    // ============================================================================
    // Edge cases and tool call tests
    // ============================================================================

    #[test]
    fn test_openai_to_anthropic_tool_calls() {
        let mut converter = OpenaiToAnthropic::default();

        // First establish state
        let _ = converter.insert(SseLine::Data(json!({
            "id": "chatcmpl-tool",
            "model": "gpt-4",
            "choices": [{ "delta": { "role": "assistant" } }]
        })));

        // Send tool call
        let chunk = json!({
            "choices": [{
                "delta": {
                    "tool_calls": [{
                        "id": "call_abc",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"location\": \"Beijing\"}"
                        }
                    }]
                }
            }]
        });

        let events = converter.insert(SseLine::Data(chunk)).unwrap();

        // Should generate tool_use content_block_start and input_json_delta
        assert!(events.is_some());
        let events = events.unwrap();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, SseLine::Event(e) if e == "content_block_start"))
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, SseLine::Event(e) if e == "content_block_delta"))
        );
    }

    #[test]
    fn test_anthropic_to_openai_tool_use() {
        let mut converter = AnthropicToOpenai::default();

        // Establish state
        let _ = converter.insert(SseLine::Data(json!({
            "type": "message_start",
            "message": {
                "id": "msg_tool",
                "model": "claude-sonnet-4-5-20250929",
                "usage": { "input_tokens": 10, "output_tokens": 0 }
            }
        })));

        // tool_use content_block_start
        let _ = converter.insert(SseLine::Data(json!({
            "type": "content_block_start",
            "index": 0,
            "content_block": {
                "type": "tool_use",
                "id": "tool_abc",
                "name": "search",
                "input": {}
            }
        })));

        // input_json_delta
        let _ = converter.insert(SseLine::Data(json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {
                "type": "input_json_delta",
                "partial_json": "{\"query\": \"weather\"}"
            }
        })));

        // content_block_stop - should generate tool_calls chunk
        let chunk = converter
            .insert(SseLine::Data(json!({
                "type": "content_block_stop",
                "index": 0
            })))
            .unwrap();

        // Should have tool_calls in chunk
        assert!(chunk.is_some());
        let lines = chunk.unwrap();
        assert!(
            lines
                .iter()
                .any(|e| matches!(e, SseLine::Data(d) if d.to_string().contains("\"tool_calls\"")))
        );
    }

    #[test]
    fn test_invalid_event_handling() {
        let mut converter = AnthropicToOpenai::default();

        // Invalid event type
        let result = converter.insert(SseLine::Data(json!({
            "type": "invalid_event_type"
        })));

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ProtocolError::InvalidStreamEvent(_)));
    }

    #[test]
    fn test_sse_line_done_variant() {
        // Test SseLine::Done handling
        let mut converter = AnthropicToOpenai::default();

        // Establish state
        let _ = converter.insert(SseLine::Data(json!({
            "type": "message_start",
            "message": { "id": "msg", "model": "claude", "usage": { "input_tokens": 0, "output_tokens": 0 } }
        })));

        // message_stop should return Done
        let result = converter
            .insert(SseLine::Data(json!({
                "type": "message_stop"
            })))
            .unwrap();

        assert!(result.is_some());
        let lines = result.unwrap();
        assert!(lines.iter().any(|e| matches!(e, SseLine::Done)));
    }
}
