//! LLM Gateway Protocols
//!
//! Protocol conversion utilities for LLM Gateway, supporting bidirectional conversion between
//! OpenAI Chat Completion API and Anthropic Messages API formats.
//!
//! # Features
//!
//! - **Request Conversion**: Convert between OpenAI and Anthropic request formats
//! - **Response Conversion**: Convert between OpenAI and Anthropic response formats
//! - **Streaming Conversion**: Real-time SSE stream conversion for both protocols
//! - **Tool Support**: Full tool/function definition and tool_choice conversion
//! - **Protocol Compliance**: Adheres to official OpenAI and Anthropic API specifications
//!
//! # Example
//!
//! ## Request Conversion (OpenAI → Anthropic)
//!
//! ```rust,ignore
//! use serde_json::json;
//! use llm_gateway_protocols::functions::request::openai_to_anthropic;
//!
//! let openai_request = json!({
//!     "model": "gpt-4",
//!     "messages": [
//!         {"role": "system", "content": "You are helpful"},
//!         {"role": "user", "content": "Hello"}
//!     ],
//!     "tools": [{
//!         "type": "function",
//!         "function": {
//!             "name": "get_weather",
//!             "parameters": {"type": "object", "properties": {"location": {"type": "string"}}}
//!         }
//!     }],
//!     "tool_choice": "auto"
//! });
//!
//! let anthropic_request = openai_to_anthropic(openai_request).unwrap();
//! // Result includes: system field, tools array with input_schema, tool_choice with type
//! ```
//!
//! ## Response Conversion (Anthropic → OpenAI)
//!
//! ```rust,ignore
//! use serde_json::json;
//! use llm_gateway_protocols::functions::response::anthropic_to_openai;
//!
//! let anthropic_response = json!({
//!     "id": "msg_abc",
//!     "type": "message",
//!     "role": "assistant",
//!     "model": "claude-sonnet-4-5-20250929",
//!     "content": [{"type": "text", "text": "Hello!"}],
//!     "stop_reason": "end_turn",
//!     "usage": {"input_tokens": 10, "output_tokens": 8}
//! });
//!
//! let openai_response = anthropic_to_openai(anthropic_response).unwrap();
//! // Result includes: choices array, finish_reason: "stop", usage with total_tokens
//! ```
//!
//! # Protocol Support Matrix
//!
//! | Feature | OpenAI → Anthropic | Anthropic → OpenAI |
//! |---------|-------------------|-------------------|
//! | Basic request/response | ✅ | ✅ |
//! | System messages | ✅ (extract to system field) | ✅ (convert to system role) |
//! | Tool definitions | ✅ (parameters → input_schema) | ✅ (input_schema → parameters) |
//! | Tool choice | ✅ (auto/none/required → type) | ✅ (type → auto/none/required) |
//! | Response format | ✅ (json_object → system hint) | - |
//! | Streaming | ✅ | ✅ |
//! | Image content | - | ⚠️ (skipped) |
//! | Document content | - | ⚠️ (skipped) |
//! | Thinking blocks | - | ⚠️ (skipped) |
//!
//! # Error Handling
//!
//! All conversion functions return `ProtocolResult<T>` which is a type alias for
//! `Result<T, ProtocolError>`. Error types include:
//!
//! - `MissingRequiredField`: Required field is missing
//! - `InvalidRequest`: Request format is invalid
//! - `ConversionError`: Conversion failed
//! - `InvalidStreamEvent`: Invalid SSE event format

mod functions;

pub use functions::request;
pub use functions::response;
pub use functions::streaming;
pub use functions::{ProtocolError, ProtocolResult};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Protocol {
    OpenAI,
    Anthropic,
}

impl Protocol {
    pub fn name(&self) -> &str {
        match self {
            Self::OpenAI => "openai",
            Self::Anthropic => "anthropic",
        }
    }
}
