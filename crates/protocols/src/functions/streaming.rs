use super::ProtocolResult;
use serde_json::Value as Json;

pub enum SseLine {
    Event(String),
    Data(Json),
}

pub trait StreamingCollector {
    fn insert(line: SseLine) -> ProtocolResult<Option<Vec<SseLine>>>;
}

pub(crate) struct OpenaiToAnthropic {
    // TODO
}

impl StreamingCollector for OpenaiToAnthropic {
    fn insert(line: SseLine) -> ProtocolResult<Option<Vec<SseLine>>> {
        todo!()
    }
}

pub(crate) struct AnthropicToOpenai {
    // TODO
}

impl StreamingCollector for AnthropicToOpenai {
    fn insert(line: SseLine) -> ProtocolResult<Option<Vec<SseLine>>> {
        todo!()
    }
}
