use super::{ProtocolError, ProtocolResult};
use serde_json::{Map, Value as Json, json};

pub fn openai_to_anthropic(body: Json) -> ProtocolResult<Json> {
    let obj = body.as_object().ok_or_else(|| {
        ProtocolError::InvalidRequest("Request body must be an object".to_string())
    })?;

    // Validate required fields using references
    let id = obj
        .get("id")
        .cloned()
        .ok_or_else(|| ProtocolError::MissingRequiredField("id".to_string()))?;

    let choices = obj
        .get("choices")
        .and_then(|v| v.as_array())
        .ok_or_else(|| ProtocolError::MissingRequiredField("choices".to_string()))?;

    if choices.is_empty() {
        return Err(ProtocolError::InvalidRequest(
            "No choices in response".to_string(),
        ));
    }

    let usage = obj
        .get("usage")
        .cloned()
        .ok_or_else(|| ProtocolError::MissingRequiredField("usage".to_string()))?;

    // Get the first choice
    let first_choice = &choices[0];
    let message = first_choice
        .get("message")
        .ok_or_else(|| ProtocolError::InvalidRequest("Choice missing message".to_string()))?;

    let finish_reason = first_choice
        .get("finish_reason")
        .and_then(|r| r.as_str())
        .unwrap_or("");

    // Convert finish_reason
    let stop_reason = match finish_reason {
        "stop" => "end_turn",
        "length" => "max_tokens",
        "tool_calls" => "tool_use",
        "content_filter" => "refusal",
        _ => finish_reason,
    };

    // Extract content and handle tool_calls
    let mut content = Vec::new();

    // Get text content if present
    if let Some(text) = message.get("content").and_then(|c| c.as_str())
        && !text.is_empty()
    {
        content.push(json!({
            "type": "text",
            "text": text
        }))
    }

    // Handle tool_calls if present
    if let Some(tool_calls) = message.get("tool_calls").and_then(|t| t.as_array()) {
        for tool_call in tool_calls {
            if let Some(function) = tool_call.get("function") {
                let tool_id = tool_call.get("id").cloned().unwrap_or(Json::Null);
                let name = function.get("name").cloned().unwrap_or(Json::Null);
                let arguments_str = function
                    .get("arguments")
                    .and_then(|a| a.as_str())
                    .unwrap_or("{}");

                // Parse arguments as JSON object with explicit error handling
                let input: Json = serde_json::from_str(arguments_str).map_err(|e| {
                    ProtocolError::ConversionError(format!("Invalid tool call arguments: {}", e))
                })?;

                content.push(json!({
                    "type": "tool_use",
                    "id": tool_id,
                    "name": name,
                    "input": input
                }))
            }
        }
    }

    // Extract usage and convert field names
    let usage_obj = usage
        .as_object()
        .ok_or_else(|| ProtocolError::InvalidRequest("usage must be an object".to_string()))?;

    let input_tokens = usage_obj
        .get("prompt_tokens")
        .cloned()
        .unwrap_or(Json::Null);
    let output_tokens = usage_obj
        .get("completion_tokens")
        .cloned()
        .unwrap_or(Json::Null);

    // Handle cache_read_input_tokens from prompt_tokens_details
    let cache_read_tokens = usage_obj
        .get("prompt_tokens_details")
        .and_then(|d| d.as_object())
        .and_then(|d| d.get("cached_tokens"))
        .cloned();

    // Build usage object
    let mut anthropic_usage = Map::new();
    anthropic_usage.insert("input_tokens".to_string(), input_tokens);
    anthropic_usage.insert("output_tokens".to_string(), output_tokens);
    if let Some(cache_tokens) = cache_read_tokens {
        anthropic_usage.insert("cache_read_input_tokens".to_string(), cache_tokens);
    }

    // Build the result using json! macro
    Ok(json!({
        "id": id,
        "type": "message",
        "role": "assistant",
        "model": obj.get("model").cloned().unwrap_or(Json::Null),
        "content": content,
        "stop_reason": if stop_reason.is_empty() { Json::Null } else { Json::String(stop_reason.to_string()) },
        "stop_sequence": Json::Null,
        "usage": anthropic_usage
    }))
}

pub fn anthropic_to_openai(body: Json) -> ProtocolResult<Json> {
    let obj = body.as_object().ok_or_else(|| {
        ProtocolError::InvalidRequest("Request body must be an object".to_string())
    })?;

    // Validate required fields
    let id = obj
        .get("id")
        .cloned()
        .ok_or_else(|| ProtocolError::MissingRequiredField("id".to_string()))?;

    let content = obj
        .get("content")
        .cloned()
        .ok_or_else(|| ProtocolError::MissingRequiredField("content".to_string()))?;

    let usage = obj
        .get("usage")
        .cloned()
        .ok_or_else(|| ProtocolError::MissingRequiredField("usage".to_string()))?;

    // Extract stop_reason and convert to finish_reason
    let stop_reason = obj
        .get("stop_reason")
        .and_then(|r| r.as_str())
        .unwrap_or("");
    let finish_reason = match stop_reason {
        "end_turn" | "stop_sequence" | "refusal" => "stop",
        "max_tokens" => "length",
        "tool_use" | "pause_turn" => "tool_calls",
        _ => stop_reason,
    };

    // Extract text content from content blocks
    let mut text_parts = Vec::new();
    let mut tool_calls = Vec::new();

    if let Some(content_array) = content.as_array() {
        for block in content_array {
            if let Some(block_type) = block.get("type").and_then(|t| t.as_str()) {
                match block_type {
                    "text" => {
                        if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                            text_parts.push(text.to_string())
                        }
                    }
                    "tool_use" => {
                        let tool_id = block.get("id").cloned().unwrap_or(Json::Null);
                        let name = block.get("name").cloned().unwrap_or(Json::Null);
                        let input = block
                            .get("input")
                            .cloned()
                            .unwrap_or(Json::Object(Map::new()));
                        // Serialize input with explicit error handling
                        let input_str = serde_json::to_string(&input).map_err(|e| {
                            ProtocolError::ConversionError(format!(
                                "Failed to serialize tool input: {}",
                                e
                            ))
                        })?;

                        tool_calls.push(json!({
                            "id": tool_id,
                            "type": "function",
                            "function": {
                                "name": name,
                                "arguments": input_str
                            }
                        }))
                    }
                    _ => {}
                }
            }
        }
    }

    // Build message content
    let message_content = if !text_parts.is_empty() {
        json!(text_parts.join(" "))
    } else {
        Json::Null
    };

    // Build message object
    let mut message = Map::new();
    message.insert("role".to_string(), json!("assistant"));
    message.insert("content".to_string(), message_content);

    if !tool_calls.is_empty() {
        message.insert("tool_calls".to_string(), Json::Array(tool_calls));
    }

    // Extract usage and convert field names
    let usage_obj = usage
        .as_object()
        .ok_or_else(|| ProtocolError::InvalidRequest("usage must be an object".to_string()))?;

    let prompt_tokens = usage_obj.get("input_tokens").cloned().unwrap_or(Json::Null);
    let output_tokens = usage_obj
        .get("output_tokens")
        .cloned()
        .unwrap_or(Json::Null);

    // Calculate total_tokens
    let total_tokens = match (&prompt_tokens, &output_tokens) {
        (Json::Number(p), Json::Number(o)) => {
            json!(p.as_i64().unwrap_or(0) + o.as_i64().unwrap_or(0))
        }
        _ => Json::Null,
    };

    // Build usage object
    let mut openai_usage = Map::new();
    openai_usage.insert("prompt_tokens".to_string(), prompt_tokens);
    openai_usage.insert("completion_tokens".to_string(), output_tokens);
    openai_usage.insert("total_tokens".to_string(), total_tokens);

    // Handle cache tokens
    let cache_read_tokens = usage_obj.get("cache_read_input_tokens").cloned();
    let cache_creation_tokens = usage_obj.get("cache_creation_input_tokens").cloned();

    if cache_read_tokens.is_some() || cache_creation_tokens.is_some() {
        let mut details = Map::new();
        if let Some(cache_read) = cache_read_tokens {
            details.insert("cached_tokens".to_string(), cache_read);
        }
        if let Some(cache_creation) = cache_creation_tokens {
            details.insert("cache_creation_tokens".to_string(), cache_creation);
        }
        openai_usage.insert("prompt_tokens_details".to_string(), Json::Object(details));
    }

    // Get current timestamp
    let created = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Build the result
    Ok(json!({
        "id": id,
        "object": "chat.completion",
        "created": created,
        "model": obj.get("model").cloned().unwrap_or(Json::Null),
        "choices": [{
            "index": 0,
            "message": message,
            "finish_reason": if finish_reason.is_empty() { Json::Null } else { Json::String(finish_reason.to_string()) }
        }],
        "usage": openai_usage
    }))
}

#[cfg(test)]
mod test {
    use super::{ProtocolError, anthropic_to_openai, openai_to_anthropic};
    use serde_json::json;

    // Helper function to normalize created timestamp for testing
    fn normalize_created(result: serde_json::Value) -> serde_json::Value {
        let mut normalized = result;
        if let Some(obj) = normalized.as_object_mut() {
            obj.insert("created".to_string(), json!(0));
        }
        normalized
    }

    // ============================================================================
    // OpenAI to Anthropic response conversion tests
    // ============================================================================

    #[test]
    fn test_openai_to_anthropic_basic_response() {
        let openai_response = json!({
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
        });

        let expected = json!({
            "id": "chatcmpl-abc123",
            "type": "message",
            "role": "assistant",
            "model": "gpt-4",
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
                "output_tokens": 8
            }
        });

        assert_eq!(openai_to_anthropic(openai_response), Ok(expected));
    }

    #[test]
    fn test_openai_to_anthropic_with_tool_calls() {
        let openai_response = json!({
            "id": "chatcmpl-tool123",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "gpt-4",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": null,
                        "tool_calls": [
                            {
                                "id": "call_abc",
                                "type": "function",
                                "function": {
                                    "name": "get_weather",
                                    "arguments": "{\"location\": \"Beijing\"}"
                                }
                            }
                        ]
                    },
                    "finish_reason": "tool_calls"
                }
            ],
            "usage": {
                "prompt_tokens": 15,
                "completion_tokens": 10,
                "total_tokens": 25
            }
        });

        let expected = json!({
            "id": "chatcmpl-tool123",
            "type": "message",
            "role": "assistant",
            "model": "gpt-4",
            "content": [
                {
                    "type": "tool_use",
                    "id": "call_abc",
                    "name": "get_weather",
                    "input": {"location": "Beijing"}
                }
            ],
            "stop_reason": "tool_use",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 15,
                "output_tokens": 10
            }
        });

        assert_eq!(openai_to_anthropic(openai_response), Ok(expected));
    }

    #[test]
    fn test_openai_to_anthropic_multiple_choices() {
        let openai_response = json!({
            "id": "chatcmpl-multi",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "gpt-4",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "First choice"
                    },
                    "finish_reason": "stop"
                },
                {
                    "index": 1,
                    "message": {
                        "role": "assistant",
                        "content": "Second choice"
                    },
                    "finish_reason": "stop"
                }
            ],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 16,
                "total_tokens": 26
            }
        });

        let expected = json!({
            "id": "chatcmpl-multi",
            "type": "message",
            "role": "assistant",
            "model": "gpt-4",
            "content": [
                {
                    "type": "text",
                    "text": "First choice"
                }
            ],
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 10,
                "output_tokens": 16
            }
        });

        assert_eq!(openai_to_anthropic(openai_response), Ok(expected));
    }

    #[test]
    fn test_openai_to_anthropic_with_usage_details() {
        let openai_response = json!({
            "id": "chatcmpl-details",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "gpt-4",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Response"
                    },
                    "finish_reason": "stop"
                }
            ],
            "usage": {
                "prompt_tokens": 20,
                "completion_tokens": 15,
                "total_tokens": 35,
                "prompt_tokens_details": {
                    "cached_tokens": 10
                },
                "completion_tokens_details": {
                    "reasoning_tokens": 5
                }
            }
        });

        let expected = json!({
            "id": "chatcmpl-details",
            "type": "message",
            "role": "assistant",
            "model": "gpt-4",
            "content": [
                {
                    "type": "text",
                    "text": "Response"
                }
            ],
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 20,
                "output_tokens": 15,
                "cache_read_input_tokens": 10
            }
        });

        assert_eq!(openai_to_anthropic(openai_response), Ok(expected));
    }

    #[test]
    fn test_openai_to_anthropic_finish_reason_stop() {
        let openai_response = json!({
            "id": "chatcmpl-stop",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "gpt-4",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Natural end"
                    },
                    "finish_reason": "stop"
                }
            ],
            "usage": {
                "prompt_tokens": 5,
                "completion_tokens": 3,
                "total_tokens": 8
            }
        });

        let expected = json!({
            "id": "chatcmpl-stop",
            "type": "message",
            "role": "assistant",
            "model": "gpt-4",
            "content": [
                {
                    "type": "text",
                    "text": "Natural end"
                }
            ],
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 5,
                "output_tokens": 3
            }
        });

        assert_eq!(openai_to_anthropic(openai_response), Ok(expected));
    }

    #[test]
    fn test_openai_to_anthropic_finish_reason_length() {
        let openai_response = json!({
            "id": "chatcmpl-length",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "gpt-4",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Truncated response..."
                    },
                    "finish_reason": "length"
                }
            ],
            "usage": {
                "prompt_tokens": 50,
                "completion_tokens": 100,
                "total_tokens": 150
            }
        });

        let expected = json!({
            "id": "chatcmpl-length",
            "type": "message",
            "role": "assistant",
            "model": "gpt-4",
            "content": [
                {
                    "type": "text",
                    "text": "Truncated response..."
                }
            ],
            "stop_reason": "max_tokens",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 50,
                "output_tokens": 100
            }
        });

        assert_eq!(openai_to_anthropic(openai_response), Ok(expected));
    }

    #[test]
    fn test_openai_to_anthropic_with_system_fingerprint() {
        let openai_response = json!({
            "id": "chatcmpl-fingerprint",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "gpt-4",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Response"
                    },
                    "finish_reason": "stop"
                }
            ],
            "usage": {
                "prompt_tokens": 5,
                "completion_tokens": 5,
                "total_tokens": 10
            },
            "system_fingerprint": "fp_abc123"
        });

        let expected = json!({
            "id": "chatcmpl-fingerprint",
            "type": "message",
            "role": "assistant",
            "model": "gpt-4",
            "content": [
                {
                    "type": "text",
                    "text": "Response"
                }
            ],
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 5,
                "output_tokens": 5
            }
        });

        assert_eq!(openai_to_anthropic(openai_response), Ok(expected));
    }

    #[test]
    fn test_openai_to_anthropic_empty_choices() {
        let openai_response = json!({
            "id": "chatcmpl-empty",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "gpt-4",
            "choices": [],
            "usage": {
                "prompt_tokens": 5,
                "completion_tokens": 0,
                "total_tokens": 5
            }
        });

        let expected_error = ProtocolError::InvalidRequest("No choices in response".to_string());
        assert_eq!(openai_to_anthropic(openai_response), Err(expected_error));
    }

    #[test]
    fn test_openai_to_anthropic_null_content() {
        let openai_response = json!({
            "id": "chatcmpl-null",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "gpt-4",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": null
                    },
                    "finish_reason": "stop"
                }
            ],
            "usage": {
                "prompt_tokens": 5,
                "completion_tokens": 0,
                "total_tokens": 5
            }
        });

        let expected = json!({
            "id": "chatcmpl-null",
            "type": "message",
            "role": "assistant",
            "model": "gpt-4",
            "content": [],
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 5,
                "output_tokens": 0
            }
        });

        assert_eq!(openai_to_anthropic(openai_response), Ok(expected));
    }

    // ============================================================================
    // Anthropic to OpenAI response conversion tests
    // ============================================================================

    #[test]
    fn test_anthropic_to_openai_basic_response() {
        let anthropic_response = json!({
            "id": "msg_01ExampleID",
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
                "output_tokens": 12
            }
        });

        let expected = json!({
            "id": "msg_01ExampleID",
            "object": "chat.completion",
            "created": 0,
            "model": "claude-sonnet-4-5-20250929",
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
                "completion_tokens": 12,
                "total_tokens": 22
            }
        });

        let result = normalize_created(anthropic_to_openai(anthropic_response).unwrap());
        assert_eq!(result, expected);
    }

    #[test]
    fn test_anthropic_to_openai_with_text_content() {
        let anthropic_response = json!({
            "id": "msg_text",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-5-20250929",
            "content": [
                {
                    "type": "text",
                    "text": "This is a text response"
                }
            ],
            "stop_reason": "stop_sequence",
            "stop_sequence": "\n\n",
            "usage": {
                "input_tokens": 15,
                "output_tokens": 20
            }
        });

        let expected = json!({
            "id": "msg_text",
            "object": "chat.completion",
            "created": 0,
            "model": "claude-sonnet-4-5-20250929",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "This is a text response"
                    },
                    "finish_reason": "stop"
                }
            ],
            "usage": {
                "prompt_tokens": 15,
                "completion_tokens": 20,
                "total_tokens": 35
            }
        });

        assert_eq!(
            normalize_created(anthropic_to_openai(anthropic_response).unwrap()),
            expected
        );
    }

    #[test]
    fn test_anthropic_to_openai_with_tool_use() {
        let anthropic_response = json!({
            "id": "msg_tool",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-5-20250929",
            "content": [
                {
                    "type": "tool_use",
                    "id": "tool_abc",
                    "name": "search",
                    "input": {"query": "weather"}
                }
            ],
            "stop_reason": "tool_use",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 20,
                "output_tokens": 15
            }
        });

        let expected = json!({
            "id": "msg_tool",
            "object": "chat.completion",
            "created": 0,
            "model": "claude-sonnet-4-5-20250929",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": null,
                        "tool_calls": [
                            {
                                "id": "tool_abc",
                                "type": "function",
                                "function": {
                                    "name": "search",
                                    "arguments": "{\"query\":\"weather\"}"
                                }
                            }
                        ]
                    },
                    "finish_reason": "tool_calls"
                }
            ],
            "usage": {
                "prompt_tokens": 20,
                "completion_tokens": 15,
                "total_tokens": 35
            }
        });

        assert_eq!(
            normalize_created(anthropic_to_openai(anthropic_response).unwrap()),
            expected
        );
    }

    #[test]
    fn test_anthropic_to_openai_multiple_content_blocks() {
        let anthropic_response = json!({
            "id": "msg_multi",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-5-20250929",
            "content": [
                {
                    "type": "text",
                    "text": "Here's the answer:"
                },
                {
                    "type": "text",
                    "text": "The result is 42"
                }
            ],
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 10,
                "output_tokens": 15
            }
        });

        let expected = json!({
            "id": "msg_multi",
            "object": "chat.completion",
            "created": 0,
            "model": "claude-sonnet-4-5-20250929",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Here's the answer: The result is 42"
                    },
                    "finish_reason": "stop"
                }
            ],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 15,
                "total_tokens": 25
            }
        });

        assert_eq!(
            normalize_created(anthropic_to_openai(anthropic_response).unwrap()),
            expected
        );
    }

    #[test]
    fn test_anthropic_to_openai_with_usage() {
        let anthropic_response = json!({
            "id": "msg_usage",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-5-20250929",
            "content": [
                {
                    "type": "text",
                    "text": "Response"
                }
            ],
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 50,
                "output_tokens": 30,
                "cache_creation_input_tokens": 100,
                "cache_read_input_tokens": 200
            }
        });

        let expected = json!({
            "id": "msg_usage",
            "object": "chat.completion",
            "created": 0,
            "model": "claude-sonnet-4-5-20250929",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Response"
                    },
                    "finish_reason": "stop"
                }
            ],
            "usage": {
                "prompt_tokens": 50,
                "completion_tokens": 30,
                "total_tokens": 80,
                "prompt_tokens_details": {
                    "cached_tokens": 200,
                    "cache_creation_tokens": 100
                }
            }
        });

        assert_eq!(
            normalize_created(anthropic_to_openai(anthropic_response).unwrap()),
            expected
        );
    }

    #[test]
    fn test_anthropic_to_openai_stop_reason_end_turn() {
        let anthropic_response = json!({
            "id": "msg_end",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-5-20250929",
            "content": [
                {
                    "type": "text",
                    "text": "Response"
                }
            ],
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 5,
                "output_tokens": 5
            }
        });

        let expected = json!({
            "id": "msg_end",
            "object": "chat.completion",
            "created": 0,
            "model": "claude-sonnet-4-5-20250929",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Response"
                    },
                    "finish_reason": "stop"
                }
            ],
            "usage": {
                "prompt_tokens": 5,
                "completion_tokens": 5,
                "total_tokens": 10
            }
        });

        assert_eq!(
            normalize_created(anthropic_to_openai(anthropic_response).unwrap()),
            expected
        );
    }

    #[test]
    fn test_anthropic_to_openai_stop_reason_max_tokens() {
        let anthropic_response = json!({
            "id": "msg_max",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-5-20250929",
            "content": [
                {
                    "type": "text",
                    "text": "Truncated..."
                }
            ],
            "stop_reason": "max_tokens",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 50,
                "output_tokens": 100
            }
        });

        let expected = json!({
            "id": "msg_max",
            "object": "chat.completion",
            "created": 0,
            "model": "claude-sonnet-4-5-20250929",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Truncated..."
                    },
                    "finish_reason": "length"
                }
            ],
            "usage": {
                "prompt_tokens": 50,
                "completion_tokens": 100,
                "total_tokens": 150
            }
        });

        assert_eq!(
            normalize_created(anthropic_to_openai(anthropic_response).unwrap()),
            expected
        );
    }

    #[test]
    fn test_anthropic_to_openai_with_cache_tokens() {
        let anthropic_response = json!({
            "id": "msg_cache",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-5-20250929",
            "content": [
                {
                    "type": "text",
                    "text": "Response"
                }
            ],
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 100,
                "output_tokens": 50,
                "cache_creation_input_tokens": 200,
                "cache_read_input_tokens": 300
            }
        });

        let expected = json!({
            "id": "msg_cache",
            "object": "chat.completion",
            "created": 0,
            "model": "claude-sonnet-4-5-20250929",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Response"
                    },
                    "finish_reason": "stop"
                }
            ],
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 50,
                "total_tokens": 150,
                "prompt_tokens_details": {
                    "cached_tokens": 300,
                    "cache_creation_tokens": 200
                }
            }
        });

        assert_eq!(
            normalize_created(anthropic_to_openai(anthropic_response).unwrap()),
            expected
        );
    }

    #[test]
    fn test_anthropic_to_openai_empty_content() {
        let anthropic_response = json!({
            "id": "msg_empty",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-5-20250929",
            "content": [],
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 5,
                "output_tokens": 0
            }
        });

        let expected = json!({
            "id": "msg_empty",
            "object": "chat.completion",
            "created": 0,
            "model": "claude-sonnet-4-5-20250929",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": null
                    },
                    "finish_reason": "stop"
                }
            ],
            "usage": {
                "prompt_tokens": 5,
                "completion_tokens": 0,
                "total_tokens": 5
            }
        });

        assert_eq!(
            normalize_created(anthropic_to_openai(anthropic_response).unwrap()),
            expected
        );
    }

    #[test]
    fn test_anthropic_to_openai_null_stop_reason() {
        let anthropic_response = json!({
            "id": "msg_null",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-5-20250929",
            "content": [
                {
                    "type": "text",
                    "text": "Response"
                }
            ],
            "stop_reason": null,
            "stop_sequence": null,
            "usage": {
                "input_tokens": 5,
                "output_tokens": 5
            }
        });

        let expected = json!({
            "id": "msg_null",
            "object": "chat.completion",
            "created": 0,
            "model": "claude-sonnet-4-5-20250929",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Response"
                    },
                    "finish_reason": null
                }
            ],
            "usage": {
                "prompt_tokens": 5,
                "completion_tokens": 5,
                "total_tokens": 10
            }
        });

        assert_eq!(
            normalize_created(anthropic_to_openai(anthropic_response).unwrap()),
            expected
        );
    }

    // ============================================================================
    // Error cases - OpenAI to Anthropic
    // ============================================================================

    #[test]
    fn test_openai_to_anthropic_missing_id() {
        let openai_response = json!({
            "object": "chat.completion",
            "created": 1234567890,
            "model": "gpt-4",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Response"
                    },
                    "finish_reason": "stop"
                }
            ],
            "usage": {
                "prompt_tokens": 5,
                "completion_tokens": 5,
                "total_tokens": 10
            }
        });

        let expected_error = ProtocolError::MissingRequiredField("id".to_string());
        assert_eq!(openai_to_anthropic(openai_response), Err(expected_error));
    }

    #[test]
    fn test_openai_to_anthropic_missing_choices() {
        let openai_response = json!({
            "id": "chatcmpl-no-choices",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "gpt-4",
            "usage": {
                "prompt_tokens": 5,
                "completion_tokens": 5,
                "total_tokens": 10
            }
        });

        let expected_error = ProtocolError::MissingRequiredField("choices".to_string());
        assert_eq!(openai_to_anthropic(openai_response), Err(expected_error));
    }

    #[test]
    fn test_openai_to_anthropic_missing_usage() {
        let openai_response = json!({
            "id": "chatcmpl-no-usage",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "gpt-4",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Response"
                    },
                    "finish_reason": "stop"
                }
            ]
        });

        let expected_error = ProtocolError::MissingRequiredField("usage".to_string());
        assert_eq!(openai_to_anthropic(openai_response), Err(expected_error));
    }

    // ============================================================================
    // Error cases - Anthropic to OpenAI
    // ============================================================================

    #[test]
    fn test_anthropic_to_openai_missing_id() {
        let anthropic_response = json!({
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-5-20250929",
            "content": [
                {
                    "type": "text",
                    "text": "Response"
                }
            ],
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 5,
                "output_tokens": 5
            }
        });

        let expected_error = ProtocolError::MissingRequiredField("id".to_string());
        assert_eq!(anthropic_to_openai(anthropic_response), Err(expected_error));
    }

    #[test]
    fn test_anthropic_to_openai_missing_content() {
        let anthropic_response = json!({
            "id": "msg-no-content",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-5-20250929",
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 5,
                "output_tokens": 5
            }
        });

        let expected_error = ProtocolError::MissingRequiredField("content".to_string());
        assert_eq!(anthropic_to_openai(anthropic_response), Err(expected_error));
    }

    #[test]
    fn test_anthropic_to_openai_missing_usage() {
        let anthropic_response = json!({
            "id": "msg-no-usage",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-5-20250929",
            "content": [
                {
                    "type": "text",
                    "text": "Response"
                }
            ],
            "stop_reason": "end_turn",
            "stop_sequence": null
        });

        let expected_error = ProtocolError::MissingRequiredField("usage".to_string());
        assert_eq!(anthropic_to_openai(anthropic_response), Err(expected_error));
    }

    // ============================================================================
    // Finish reason mapping tests - OpenAI to Anthropic
    // ============================================================================

    #[test]
    fn test_openai_to_anthropic_finish_reason_content_filter() {
        let openai_response = json!({
            "id": "chatcmpl-filter",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "gpt-4",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Response"
                    },
                    "finish_reason": "content_filter"
                }
            ],
            "usage": {
                "prompt_tokens": 5,
                "completion_tokens": 5,
                "total_tokens": 10
            }
        });

        let expected = json!({
            "id": "chatcmpl-filter",
            "type": "message",
            "role": "assistant",
            "model": "gpt-4",
            "content": [
                {
                    "type": "text",
                    "text": "Response"
                }
            ],
            "stop_reason": "refusal",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 5,
                "output_tokens": 5
            }
        });

        assert_eq!(openai_to_anthropic(openai_response), Ok(expected));
    }

    // ============================================================================
    // Finish reason mapping tests - Anthropic to OpenAI
    // ============================================================================

    #[test]
    fn test_anthropic_to_openai_stop_reason_pause_turn() {
        let anthropic_response = json!({
            "id": "msg_pause",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-5-20250929",
            "content": [
                {
                    "type": "tool_use",
                    "id": "tool_abc",
                    "name": "long_running_tool",
                    "input": {}
                }
            ],
            "stop_reason": "pause_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 10,
                "output_tokens": 5
            }
        });

        let expected = json!({
            "id": "msg_pause",
            "object": "chat.completion",
            "created": 0,
            "model": "claude-sonnet-4-5-20250929",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": null,
                        "tool_calls": [
                            {
                                "id": "tool_abc",
                                "type": "function",
                                "function": {
                                    "name": "long_running_tool",
                                    "arguments": "{}"
                                }
                            }
                        ]
                    },
                    "finish_reason": "tool_calls"
                }
            ],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        });

        assert_eq!(
            normalize_created(anthropic_to_openai(anthropic_response).unwrap()),
            expected
        );
    }

    #[test]
    fn test_anthropic_to_openai_stop_reason_refusal() {
        let anthropic_response = json!({
            "id": "msg_refusal",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-5-20250929",
            "content": [
                {
                    "type": "text",
                    "text": "I cannot help with that."
                }
            ],
            "stop_reason": "refusal",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 10,
                "output_tokens": 8
            }
        });

        let expected = json!({
            "id": "msg_refusal",
            "object": "chat.completion",
            "created": 0,
            "model": "claude-sonnet-4-5-20250929",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "I cannot help with that."
                    },
                    "finish_reason": "stop"
                }
            ],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 8,
                "total_tokens": 18
            }
        });

        assert_eq!(
            normalize_created(anthropic_to_openai(anthropic_response).unwrap()),
            expected
        );
    }

    // ============================================================================
    // Cache tokens mapping tests
    // ============================================================================

    #[test]
    fn test_anthropic_to_openai_with_cache_creation_tokens() {
        let anthropic_response = json!({
            "id": "msg_cache_create",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-5-20250929",
            "content": [
                {
                    "type": "text",
                    "text": "Response"
                }
            ],
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "usage": {
                "input_tokens": 100,
                "output_tokens": 50,
                "cache_creation_input_tokens": 200,
                "cache_read_input_tokens": 300
            }
        });

        let expected = json!({
            "id": "msg_cache_create",
            "object": "chat.completion",
            "created": 0,
            "model": "claude-sonnet-4-5-20250929",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Response"
                    },
                    "finish_reason": "stop"
                }
            ],
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 50,
                "total_tokens": 150,
                "prompt_tokens_details": {
                    "cached_tokens": 300,
                    "cache_creation_tokens": 200
                }
            }
        });

        assert_eq!(
            normalize_created(anthropic_to_openai(anthropic_response).unwrap()),
            expected
        );
    }

    // ============================================================================
    // Stop sequence tests
    // ============================================================================

    #[test]
    fn test_anthropic_to_openai_with_stop_sequence() {
        let anthropic_response = json!({
            "id": "msg_stop_seq",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-5-20250929",
            "content": [
                {
                    "type": "text",
                    "text": "Response ended"
                }
            ],
            "stop_reason": "stop_sequence",
            "stop_sequence": "\n\n",
            "usage": {
                "input_tokens": 10,
                "output_tokens": 5
            }
        });

        let expected = json!({
            "id": "msg_stop_seq",
            "object": "chat.completion",
            "created": 0,
            "model": "claude-sonnet-4-5-20250929",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Response ended"
                    },
                    "finish_reason": "stop"
                }
            ],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        });

        assert_eq!(
            normalize_created(anthropic_to_openai(anthropic_response).unwrap()),
            expected
        );
    }
}
