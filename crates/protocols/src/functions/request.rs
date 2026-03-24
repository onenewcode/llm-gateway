use super::{ProtocolError, ProtocolResult};
use serde_json::{Value as Json, json};

pub(crate) fn openai_to_anthropic(body: Json) -> ProtocolResult<Json> {
    let obj = body.as_object().ok_or_else(|| {
        ProtocolError::InvalidRequest("Request body must be an object".to_string())
    })?;

    // Validate required fields using references first
    let model = obj
        .get("model")
        .cloned()
        .ok_or_else(|| ProtocolError::MissingRequiredField("model".to_string()))?;

    let messages = obj
        .get("messages")
        .and_then(|v| v.as_array())
        .ok_or_else(|| ProtocolError::MissingRequiredField("messages".to_string()))?;

    // Validate message roles and collect system prompts using references
    let mut system_prompts = Vec::new();
    let mut filtered_messages = Vec::new();

    for msg in messages {
        let role = msg
            .get("role")
            .and_then(|r| r.as_str())
            .ok_or_else(|| ProtocolError::InvalidRequest("Message missing role".to_string()))?;

        match role {
            "system" => {
                if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
                    system_prompts.push(content.to_string())
                }
            }
            "user" | "assistant" | "tool" => {
                // Validate that message has content or tool_calls
                let has_content = msg.get("content").is_some();
                let has_tool_calls = msg.get("tool_calls").is_some();

                if !has_content && !has_tool_calls {
                    return Err(ProtocolError::InvalidRequest(
                        "Message must have content or tool_calls".to_string(),
                    ));
                }

                // Clone only valid messages
                filtered_messages.push(msg.clone())
            }
            _ => {
                return Err(ProtocolError::InvalidRequest(format!(
                    "Invalid message role: {}",
                    role
                )));
            }
        }
    }

    // Build max_tokens value
    let max_tokens_val = obj
        .get("max_tokens")
        .cloned()
        .unwrap_or_else(|| json!(2048));

    // Build stop_sequences value
    let stop_sequences_val = obj.get("stop").map(|stop| {
        if stop.is_string() {
            json!([stop.as_str().unwrap()])
        } else if stop.is_array() {
            stop.clone()
        } else {
            json!([])
        }
    });

    // Build the result using json! macro
    let mut result = json!({
        "model": model,
        "messages": filtered_messages,
        "max_tokens": max_tokens_val
    });

    // Add system if there are system prompts
    if !system_prompts.is_empty() {
        result["system"] = json!(system_prompts.join(" "))
    }

    // Copy over optional fields
    if let Some(temp) = obj.get("temperature") {
        result["temperature"] = temp.clone()
    }
    if let Some(top_p) = obj.get("top_p") {
        result["top_p"] = top_p.clone()
    }
    if let Some(stop_seq) = stop_sequences_val {
        result["stop_sequences"] = stop_seq
    }
    if let Some(freq_penalty) = obj.get("frequency_penalty") {
        result["frequency_penalty"] = freq_penalty.clone()
    }
    if let Some(pres_penalty) = obj.get("presence_penalty") {
        result["presence_penalty"] = pres_penalty.clone()
    }
    if let Some(stream) = obj.get("stream") {
        result["stream"] = stream.clone()
    }
    if let Some(stream_options) = obj.get("stream_options") {
        result["stream_options"] = stream_options.clone()
    }

    // Convert tools if present
    if let Some(tools) = obj.get("tools").and_then(|v| v.as_array()) {
        let converted_tools: Vec<_> = tools.iter().filter_map(convert_openai_tool).collect();
        if !converted_tools.is_empty() {
            result["tools"] = json!(converted_tools);
        }
    }

    // Convert tool_choice if present
    if let Some(tool_choice) = obj.get("tool_choice")
        && let Some(converted) = convert_openai_tool_choice(tool_choice)
    {
        result["tool_choice"] = converted;
    }

    // Convert response_format if present
    if let Some(response_format) = obj.get("response_format")
        && let Some(format_type) = response_format.get("type").and_then(|v| v.as_str())
        && format_type == "json_object"
    {
        // Append JSON format instruction to system prompt
        let json_instruction = " Please respond in JSON format.";
        if let Some(system) = result.get("system").and_then(|v| v.as_str()) {
            result["system"] = json!(format!("{}{}", system, json_instruction));
        } else {
            result["system"] = json!(json_instruction.trim());
        }
    }

    // Note: seed and user fields are dropped (no Anthropic equivalent)

    Ok(result)
}

/// Convert OpenAI tool definition to Anthropic format
fn convert_openai_tool(tool: &Json) -> Option<Json> {
    let obj = tool.as_object()?;

    // Handle OpenAI tool format
    if let Some(function) = obj.get("function").and_then(|v| v.as_object()) {
        let name = function.get("name")?.as_str()?.to_string();
        let description = function
            .get("description")
            .and_then(|v| v.as_str())
            .map(String::from);

        // OpenAI parameters is JSON Schema, directly map to input_schema
        let input_schema = function
            .get("parameters")
            .cloned()
            .unwrap_or_else(|| json!({"type": "object"}));

        Some(json!({
            "name": name,
            "description": description,
            "input_schema": input_schema
        }))
    } else {
        // Pass through if already in Anthropic format
        Some(tool.clone())
    }
}

/// Convert OpenAI tool_choice to Anthropic format
fn convert_openai_tool_choice(tool_choice: &Json) -> Option<Json> {
    // Handle string format
    if let Some(choice_str) = tool_choice.as_str() {
        return match choice_str {
            "auto" => Some(json!({"type": "auto"})),
            "none" => Some(json!({"type": "none"})),
            "required" => Some(json!({"type": "any"})),
            _ => None,
        };
    }

    // Handle object format
    if let Some(obj) = tool_choice.as_object()
        && let Some(choice_type) = obj.get("type").and_then(|v| v.as_str())
        && choice_type == "function"
        && let Some(function) = obj.get("function").and_then(|v| v.as_object())
        && let Some(name) = function.get("name").and_then(|v| v.as_str())
    {
        return Some(json!({"type": "tool", "name": name}));
    }

    None
}

pub(crate) fn anthropic_to_openai(body: Json) -> ProtocolResult<Json> {
    let obj = body.as_object().ok_or_else(|| {
        ProtocolError::InvalidRequest("Request body must be an object".to_string())
    })?;

    // Validate required fields using references first
    let model = obj
        .get("model")
        .cloned()
        .ok_or_else(|| ProtocolError::MissingRequiredField("model".to_string()))?;

    let max_tokens = obj
        .get("max_tokens")
        .cloned()
        .ok_or_else(|| ProtocolError::MissingRequiredField("max_tokens".to_string()))?;

    let messages = obj
        .get("messages")
        .and_then(|v| v.as_array())
        .ok_or_else(|| ProtocolError::MissingRequiredField("messages".to_string()))?;

    // Validate message roles using references
    for msg in messages {
        let role = msg
            .get("role")
            .and_then(|r| r.as_str())
            .ok_or_else(|| ProtocolError::InvalidRequest("Message missing role".to_string()))?;

        if role != "user" && role != "assistant" {
            return Err(ProtocolError::InvalidRequest(format!(
                "Invalid message role: {}",
                role
            )));
        }

        // Validate that message has content
        if msg.get("content").is_none() {
            return Err(ProtocolError::InvalidRequest(
                "Message must have content".to_string(),
            ));
        }
    }

    // Build messages list
    let mut openai_messages = Vec::new();

    // Add system message first if present
    if let Some(system) = obj.get("system") {
        let system_content = if system.is_string() {
            system.as_str().unwrap().to_string()
        } else if system.is_array() {
            // Handle system as array of text blocks
            let mut text_parts = Vec::new();
            for block in system.as_array().unwrap() {
                if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                    text_parts.push(text.to_string())
                }
            }
            text_parts.join(" ")
        } else {
            String::new()
        };

        if !system_content.is_empty() {
            openai_messages.push(json!({
                "role": "system",
                "content": system_content
            }))
        }
    }

    // Add user/assistant messages
    for msg in messages {
        openai_messages.push(msg.clone())
    }

    // Build the result using json! macro
    let mut result = json!({
        "model": model,
        "max_tokens": max_tokens,
        "messages": openai_messages
    });

    // Copy over optional fields
    if let Some(temp) = obj.get("temperature") {
        result["temperature"] = temp.clone()
    }
    if let Some(top_p) = obj.get("top_p") {
        result["top_p"] = top_p.clone()
    }
    if let Some(top_k) = obj.get("top_k") {
        result["top_k"] = top_k.clone()
    }
    if let Some(stop) = obj.get("stop_sequences") {
        result["stop"] = stop.clone()
    }
    if let Some(stream) = obj.get("stream") {
        result["stream"] = stream.clone()
    }

    // Convert tools if present
    if let Some(tools) = obj.get("tools").and_then(|v| v.as_array()) {
        let converted_tools: Vec<_> = tools.iter().filter_map(convert_anthropic_tool).collect();
        if !converted_tools.is_empty() {
            result["tools"] = json!(converted_tools);
        }
    }

    // Convert tool_choice if present
    if let Some(tool_choice) = obj.get("tool_choice")
        && let Some(converted) = convert_anthropic_tool_choice(tool_choice)
    {
        result["tool_choice"] = converted;
    }

    // Note: seed and user fields have no Anthropic equivalent
    // Note: Anthropic metadata is dropped

    Ok(result)
}

/// Convert Anthropic tool definition to OpenAI format
fn convert_anthropic_tool(tool: &Json) -> Option<Json> {
    let obj = tool.as_object()?;

    // Check if it's already in OpenAI format (pass through)
    if obj.get("function").is_some() {
        return Some(tool.clone());
    }

    // Handle Anthropic tool format
    let name = obj.get("name")?.as_str()?.to_string();
    let description = obj
        .get("description")
        .and_then(|v| v.as_str())
        .map(String::from);

    // Anthropic input_schema maps to OpenAI parameters
    let parameters = obj
        .get("input_schema")
        .cloned()
        .unwrap_or_else(|| json!({"type": "object"}));

    Some(json!({
        "type": "function",
        "function": {
            "name": name,
            "description": description,
            "parameters": parameters
        }
    }))
}

/// Convert Anthropic tool_choice to OpenAI format
fn convert_anthropic_tool_choice(tool_choice: &Json) -> Option<Json> {
    if let Some(obj) = tool_choice.as_object()
        && let Some(choice_type) = obj.get("type").and_then(|v| v.as_str())
    {
        return match choice_type {
            "auto" => Some(json!("auto")),
            "none" => Some(json!("none")),
            "any" => Some(json!("required")),
            "tool" => obj.get("name").and_then(|v| v.as_str()).map(|name| {
                json!({
                    "type": "function",
                    "function": {"name": name}
                })
            }),
            _ => None,
        };
    }
    None
}

#[cfg(test)]
mod test {
    use super::{ProtocolError, anthropic_to_openai, openai_to_anthropic};
    use serde_json::json;

    // ============================================================================
    // OpenAI to Anthropic conversion tests
    // ============================================================================

    #[test]
    fn test_openai_to_anthropic_basic_request() {
        let openai_request = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Hello, how are you?"}
            ],
            "temperature": 0.7,
            "max_tokens": 100
        });

        let expected = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Hello, how are you?"}
            ],
            "temperature": 0.7,
            "max_tokens": 100
        });

        assert_eq!(openai_to_anthropic(openai_request), Ok(expected))
    }

    #[test]
    fn test_openai_to_anthropic_with_system_message() {
        let openai_request = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "system", "content": "You are a helpful assistant."},
                {"role": "user", "content": "Hello!"}
            ],
            "temperature": 0.7
        });

        let expected = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Hello!"}
            ],
            "temperature": 0.7,
            "system": "You are a helpful assistant.",
            "max_tokens": 2048
        });

        assert_eq!(openai_to_anthropic(openai_request), Ok(expected))
    }

    #[test]
    fn test_openai_to_anthropic_with_multiple_system_messages() {
        let openai_request = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "system", "content": "First system prompt."},
                {"role": "user", "content": "Hi"},
                {"role": "system", "content": "Second system prompt."},
                {"role": "assistant", "content": "Hello!"},
                {"role": "user", "content": "How are you?"}
            ]
        });

        let expected = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Hi"},
                {"role": "assistant", "content": "Hello!"},
                {"role": "user", "content": "How are you?"}
            ],
            "system": "First system prompt. Second system prompt.",
            "max_tokens": 2048
        });

        assert_eq!(openai_to_anthropic(openai_request), Ok(expected))
    }

    #[test]
    fn test_openai_to_anthropic_conversational_history() {
        let openai_request = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "system", "content": "You are a coding assistant."},
                {"role": "user", "content": "Write a function"},
                {"role": "assistant", "content": "Sure, here it is: ```python\ndef hello(): pass```"},
                {"role": "user", "content": "Can you explain it?"}
            ],
            "temperature": 0.5,
            "max_tokens": 200,
            "top_p": 0.9
        });

        let expected = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Write a function"},
                {"role": "assistant", "content": "Sure, here it is: ```python\ndef hello(): pass```"},
                {"role": "user", "content": "Can you explain it?"}
            ],
            "temperature": 0.5,
            "max_tokens": 200,
            "top_p": 0.9,
            "system": "You are a coding assistant."
        });

        assert_eq!(openai_to_anthropic(openai_request), Ok(expected))
    }

    #[test]
    fn test_openai_to_anthropic_streaming_request() {
        let openai_request = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Count from 1 to 10"}
            ],
            "stream": true,
            "stream_options": {
                "include_usage": true
            },
            "temperature": 0.7
        });

        let expected = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Count from 1 to 10"}
            ],
            "stream": true,
            "stream_options": {
                "include_usage": true
            },
            "temperature": 0.7,
            "max_tokens": 2048
        });

        assert_eq!(openai_to_anthropic(openai_request), Ok(expected))
    }

    #[test]
    fn test_openai_to_anthropic_with_stop_sequences() {
        let openai_request = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "List items"}
            ],
            "stop": ["###", "END"]
        });

        let expected = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "List items"}
            ],
            "stop_sequences": ["###", "END"],
            "max_tokens": 2048
        });

        assert_eq!(openai_to_anthropic(openai_request), Ok(expected))
    }

    #[test]
    fn test_openai_to_anthropic_with_single_stop_sequence() {
        let openai_request = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Generate text"}
            ],
            "stop": "\n\n"
        });

        let expected = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Generate text"}
            ],
            "stop_sequences": ["\n\n"],
            "max_tokens": 2048
        });

        assert_eq!(openai_to_anthropic(openai_request), Ok(expected))
    }

    #[test]
    fn test_openai_to_anthropic_with_frequency_and_presence_penalty() {
        let openai_request = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Write creatively"}
            ],
            "frequency_penalty": 0.5,
            "presence_penalty": 0.3
        });

        let expected = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Write creatively"}
            ],
            "frequency_penalty": 0.5,
            "presence_penalty": 0.3,
            "max_tokens": 2048
        });

        assert_eq!(openai_to_anthropic(openai_request), Ok(expected))
    }

    #[test]
    fn test_openai_to_anthropic_missing_max_tokens_uses_default() {
        let openai_request = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Say something"}
            ]
        });

        let expected = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Say something"}
            ],
            "max_tokens": 2048
        });

        assert_eq!(openai_to_anthropic(openai_request), Ok(expected))
    }

    // ============================================================================
    // Anthropic to OpenAI conversion tests
    // ============================================================================

    #[test]
    fn test_anthropic_to_openai_basic_request() {
        let anthropic_request = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 1024,
            "messages": [
                {"role": "user", "content": "Hello, Claude!"}
            ]
        });

        let expected = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 1024,
            "messages": [
                {"role": "user", "content": "Hello, Claude!"}
            ]
        });

        assert_eq!(anthropic_to_openai(anthropic_request), Ok(expected))
    }

    #[test]
    fn test_anthropic_to_openai_with_system_field() {
        let anthropic_request = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 1024,
            "system": "You are a helpful coding assistant.",
            "messages": [
                {"role": "user", "content": "Write a Python function"}
            ]
        });

        let expected = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 1024,
            "messages": [
                {"role": "system", "content": "You are a helpful coding assistant."},
                {"role": "user", "content": "Write a Python function"}
            ]
        });

        assert_eq!(anthropic_to_openai(anthropic_request), Ok(expected))
    }

    #[test]
    fn test_anthropic_to_openai_with_string_system_as_text_block() {
        let anthropic_request = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 1024,
            "system": [
                {"type": "text", "text": "You are a helpful assistant."}
            ],
            "messages": [
                {"role": "user", "content": "Hello"}
            ]
        });

        let expected = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 1024,
            "messages": [
                {"role": "system", "content": "You are a helpful assistant."},
                {"role": "user", "content": "Hello"}
            ]
        });

        assert_eq!(anthropic_to_openai(anthropic_request), Ok(expected))
    }

    #[test]
    fn test_anthropic_to_openai_conversational_history() {
        let anthropic_request = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 1024,
            "messages": [
                {"role": "user", "content": "Write a function"},
                {"role": "assistant", "content": "Here is the code:"},
                {"role": "user", "content": "Can you optimize it?"}
            ],
            "temperature": 0.5,
            "top_p": 0.9
        });

        let expected = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 1024,
            "messages": [
                {"role": "user", "content": "Write a function"},
                {"role": "assistant", "content": "Here is the code:"},
                {"role": "user", "content": "Can you optimize it?"}
            ],
            "temperature": 0.5,
            "top_p": 0.9
        });

        assert_eq!(anthropic_to_openai(anthropic_request), Ok(expected))
    }

    #[test]
    fn test_anthropic_to_openai_with_stop_sequences() {
        let anthropic_request = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 512,
            "messages": [
                {"role": "user", "content": "Generate list"}
            ],
            "stop_sequences": ["###", "END"]
        });

        let expected = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 512,
            "messages": [
                {"role": "user", "content": "Generate list"}
            ],
            "stop": ["###", "END"]
        });

        assert_eq!(anthropic_to_openai(anthropic_request), Ok(expected))
    }

    #[test]
    fn test_anthropic_to_openai_streaming_request() {
        let anthropic_request = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 1024,
            "messages": [
                {"role": "user", "content": "Tell me a story"}
            ],
            "stream": true
        });

        let expected = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 1024,
            "messages": [
                {"role": "user", "content": "Tell me a story"}
            ],
            "stream": true
        });

        assert_eq!(anthropic_to_openai(anthropic_request), Ok(expected))
    }

    #[test]
    fn test_anthropic_to_openai_with_top_k() {
        let anthropic_request = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 256,
            "messages": [
                {"role": "user", "content": "Generate"}
            ],
            "top_k": 50
        });

        let expected = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 256,
            "messages": [
                {"role": "user", "content": "Generate"}
            ],
            "top_k": 50
        });

        assert_eq!(anthropic_to_openai(anthropic_request), Ok(expected))
    }

    #[test]
    fn test_anthropic_to_openai_preserves_all_optional_fields() {
        let anthropic_request = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 1024,
            "messages": [
                {"role": "user", "content": "Test"}
            ],
            "temperature": 0.8,
            "top_p": 0.95,
            "top_k": 40,
            "stop_sequences": ["\n\n", "END"],
            "system": "Be concise"
        });

        let expected = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 1024,
            "messages": [
                {"role": "system", "content": "Be concise"},
                {"role": "user", "content": "Test"}
            ],
            "temperature": 0.8,
            "top_p": 0.95,
            "top_k": 40,
            "stop": ["\n\n", "END"]
        });

        assert_eq!(anthropic_to_openai(anthropic_request), Ok(expected))
    }

    // ============================================================================
    // Edge cases and error handling
    // ============================================================================

    #[test]
    fn test_openai_to_anthropic_empty_messages_array() {
        let openai_request = json!({
            "model": "gpt-4",
            "messages": []
        });

        let expected = json!({
            "model": "gpt-4",
            "messages": [],
            "max_tokens": 2048
        });

        assert_eq!(openai_to_anthropic(openai_request), Ok(expected))
    }

    #[test]
    fn test_anthropic_to_openai_empty_messages_array() {
        let anthropic_request = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 1024,
            "messages": []
        });

        let expected = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 1024,
            "messages": []
        });

        assert_eq!(anthropic_to_openai(anthropic_request), Ok(expected))
    }

    #[test]
    fn test_openai_to_anthropic_null_content() {
        let openai_request = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": null}
            ]
        });

        let expected = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": null}
            ],
            "max_tokens": 2048
        });

        assert_eq!(openai_to_anthropic(openai_request), Ok(expected))
    }

    #[test]
    fn test_openai_to_anthropic_with_name_field() {
        let openai_request = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Hello", "name": "Alice"}
            ]
        });

        let expected = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Hello", "name": "Alice"}
            ],
            "max_tokens": 2048
        });

        assert_eq!(openai_to_anthropic(openai_request), Ok(expected))
    }

    // ============================================================================
    // Error cases - OpenAI to Anthropic
    // ============================================================================

    #[test]
    fn test_openai_to_anthropic_missing_model() {
        let openai_request = json!({
            "messages": [
                {"role": "user", "content": "Hello"}
            ]
        });

        let expected_error = ProtocolError::MissingRequiredField("model".to_string());
        assert_eq!(openai_to_anthropic(openai_request), Err(expected_error))
    }

    #[test]
    fn test_openai_to_anthropic_missing_messages() {
        let openai_request = json!({
            "model": "gpt-4"
        });

        let expected_error = ProtocolError::MissingRequiredField("messages".to_string());
        assert_eq!(openai_to_anthropic(openai_request), Err(expected_error))
    }

    #[test]
    fn test_openai_to_anthropic_invalid_message_role() {
        let openai_request = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "invalid_role", "content": "Hello"}
            ]
        });

        let expected_error =
            ProtocolError::InvalidRequest("Invalid message role: invalid_role".to_string());
        assert_eq!(openai_to_anthropic(openai_request), Err(expected_error))
    }

    #[test]
    fn test_openai_to_anthropic_message_without_content_or_tool_calls() {
        let openai_request = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "assistant"}
            ]
        });

        let expected_error =
            ProtocolError::InvalidRequest("Message must have content or tool_calls".to_string());
        assert_eq!(openai_to_anthropic(openai_request), Err(expected_error))
    }

    // ============================================================================
    // Error cases - Anthropic to OpenAI
    // ============================================================================

    #[test]
    fn test_anthropic_to_openai_missing_model() {
        let anthropic_request = json!({
            "max_tokens": 1024,
            "messages": [
                {"role": "user", "content": "Hello"}
            ]
        });

        let expected_error = ProtocolError::MissingRequiredField("model".to_string());
        assert_eq!(anthropic_to_openai(anthropic_request), Err(expected_error))
    }

    #[test]
    fn test_anthropic_to_openai_missing_max_tokens() {
        let anthropic_request = json!({
            "model": "claude-sonnet-4-5-20250929",
            "messages": [
                {"role": "user", "content": "Hello"}
            ]
        });

        let expected_error = ProtocolError::MissingRequiredField("max_tokens".to_string());
        assert_eq!(anthropic_to_openai(anthropic_request), Err(expected_error))
    }

    #[test]
    fn test_anthropic_to_openai_missing_messages() {
        let anthropic_request = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 1024
        });

        let expected_error = ProtocolError::MissingRequiredField("messages".to_string());
        assert_eq!(anthropic_to_openai(anthropic_request), Err(expected_error))
    }

    #[test]
    fn test_anthropic_to_openai_invalid_message_role() {
        let anthropic_request = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 1024,
            "messages": [
                {"role": "system", "content": "Hello"}
            ]
        });

        let expected_error =
            ProtocolError::InvalidRequest("Invalid message role: system".to_string());
        assert_eq!(anthropic_to_openai(anthropic_request), Err(expected_error))
    }

    #[test]
    fn test_anthropic_to_openai_message_without_content() {
        let anthropic_request = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 1024,
            "messages": [
                {"role": "user"}
            ]
        });

        let expected_error = ProtocolError::InvalidRequest("Message must have content".to_string());
        assert_eq!(anthropic_to_openai(anthropic_request), Err(expected_error))
    }

    // ============================================================================
    // Additional field mapping tests
    // ============================================================================

    #[test]
    fn test_openai_to_anthropic_with_tool_role_message() {
        let openai_request = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "What's the weather?"},
                {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_abc",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"location\": \"Beijing\"}"
                        }
                    }]
                },
                {
                    "role": "tool",
                    "tool_call_id": "call_abc",
                    "content": "{\"temp\": 25}"
                }
            ]
        });

        let expected = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "What's the weather?"},
                {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_abc",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"location\": \"Beijing\"}"
                        }
                    }]
                },
                {
                    "role": "tool",
                    "tool_call_id": "call_abc",
                    "content": "{\"temp\": 25}"
                }
            ],
            "max_tokens": 2048
        });

        assert_eq!(openai_to_anthropic(openai_request), Ok(expected))
    }

    #[test]
    fn test_anthropic_to_openai_preserves_stop_sequences() {
        let anthropic_request = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 1024,
            "messages": [
                {"role": "user", "content": "Generate text"}
            ],
            "stop_sequences": ["###", "END"]
        });

        let expected = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 1024,
            "messages": [
                {"role": "user", "content": "Generate text"}
            ],
            "stop": ["###", "END"]
        });

        assert_eq!(anthropic_to_openai(anthropic_request), Ok(expected))
    }

    // ============================================================================
    // Tool definition conversion tests
    // ============================================================================

    #[test]
    fn test_openai_to_anthropic_tools_conversion() {
        let openai_request = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Get weather"}
            ],
            "tools": [{
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "description": "Get current weather",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "location": {"type": "string"}
                        },
                        "required": ["location"]
                    }
                }
            }]
        });

        let expected = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Get weather"}
            ],
            "max_tokens": 2048,
            "tools": [{
                "name": "get_weather",
                "description": "Get current weather",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "location": {"type": "string"}
                    },
                    "required": ["location"]
                }
            }]
        });

        assert_eq!(openai_to_anthropic(openai_request), Ok(expected))
    }

    #[test]
    fn test_anthropic_to_openai_tools_conversion() {
        let anthropic_request = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 1024,
            "messages": [
                {"role": "user", "content": "Get weather"}
            ],
            "tools": [{
                "name": "get_weather",
                "description": "Get current weather",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "location": {"type": "string"}
                    },
                    "required": ["location"]
                }
            }]
        });

        let expected = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 1024,
            "messages": [
                {"role": "user", "content": "Get weather"}
            ],
            "tools": [{
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "description": "Get current weather",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "location": {"type": "string"}
                        },
                        "required": ["location"]
                    }
                }
            }]
        });

        assert_eq!(anthropic_to_openai(anthropic_request), Ok(expected))
    }

    // ============================================================================
    // Tool choice conversion tests
    // ============================================================================

    #[test]
    fn test_openai_to_anthropic_tool_choice_auto() {
        let openai_request = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Test"}],
            "tool_choice": "auto"
        });

        let expected = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Test"}],
            "max_tokens": 2048,
            "tool_choice": {"type": "auto"}
        });

        assert_eq!(openai_to_anthropic(openai_request), Ok(expected))
    }

    #[test]
    fn test_openai_to_anthropic_tool_choice_none() {
        let openai_request = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Test"}],
            "tool_choice": "none"
        });

        let expected = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Test"}],
            "max_tokens": 2048,
            "tool_choice": {"type": "none"}
        });

        assert_eq!(openai_to_anthropic(openai_request), Ok(expected))
    }

    #[test]
    fn test_openai_to_anthropic_tool_choice_required() {
        let openai_request = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Test"}],
            "tool_choice": "required"
        });

        let expected = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Test"}],
            "max_tokens": 2048,
            "tool_choice": {"type": "any"}
        });

        assert_eq!(openai_to_anthropic(openai_request), Ok(expected))
    }

    #[test]
    fn test_openai_to_anthropic_tool_choice_specific() {
        let openai_request = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Test"}],
            "tool_choice": {
                "type": "function",
                "function": {"name": "get_weather"}
            }
        });

        let expected = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Test"}],
            "max_tokens": 2048,
            "tool_choice": {"type": "tool", "name": "get_weather"}
        });

        assert_eq!(openai_to_anthropic(openai_request), Ok(expected))
    }

    #[test]
    fn test_anthropic_to_openai_tool_choice_auto() {
        let anthropic_request = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Test"}],
            "tool_choice": {"type": "auto"}
        });

        let expected = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Test"}],
            "tool_choice": "auto"
        });

        assert_eq!(anthropic_to_openai(anthropic_request), Ok(expected))
    }

    #[test]
    fn test_anthropic_to_openai_tool_choice_any() {
        let anthropic_request = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Test"}],
            "tool_choice": {"type": "any"}
        });

        let expected = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Test"}],
            "tool_choice": "required"
        });

        assert_eq!(anthropic_to_openai(anthropic_request), Ok(expected))
    }

    #[test]
    fn test_anthropic_to_openai_tool_choice_specific() {
        let anthropic_request = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Test"}],
            "tool_choice": {"type": "tool", "name": "get_weather"}
        });

        let expected = json!({
            "model": "claude-sonnet-4-5-20250929",
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": "Test"}],
            "tool_choice": {
                "type": "function",
                "function": {"name": "get_weather"}
            }
        });

        assert_eq!(anthropic_to_openai(anthropic_request), Ok(expected))
    }

    // ============================================================================
    // Response format conversion tests
    // ============================================================================

    #[test]
    fn test_openai_to_anthropic_response_format_json_object() {
        let openai_request = json!({
            "model": "gpt-4",
            "messages": [{"role": "user", "content": "Generate JSON"}],
            "response_format": {"type": "json_object"}
        });

        let result = openai_to_anthropic(openai_request).unwrap();
        assert!(result.get("system").is_some());
        assert!(result["system"].as_str().unwrap().contains("JSON"));
    }

    #[test]
    fn test_openai_to_anthropic_response_format_json_object_with_existing_system() {
        let openai_request = json!({
            "model": "gpt-4",
            "messages": [
                {"role": "system", "content": "You are helpful"},
                {"role": "user", "content": "Generate JSON"}
            ],
            "response_format": {"type": "json_object"}
        });

        let result = openai_to_anthropic(openai_request).unwrap();
        assert!(
            result["system"]
                .as_str()
                .unwrap()
                .contains("You are helpful")
        );
        assert!(result["system"].as_str().unwrap().contains("JSON"));
    }
}
