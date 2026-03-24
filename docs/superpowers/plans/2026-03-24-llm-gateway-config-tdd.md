# llm-gateway-config 实现计划

> **对于代理工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 来逐个任务实现此计划。步骤使用复选框（`- [ ]`）语法进行跟踪。

**目标：** 实现一个基于 TOML 的配置文件解析器，将 config.toml 转换为强类型 Rust 结构体。

**架构：** 使用严格 TDD 方法：首先编写所有失败的测试（包括输入节点、虚拟节点、后端节点、错误处理），然后实现最小代码使所有测试通过。解析器将 TOML 反序列化为带有适当错误处理的 GatewayConfig。

**技术栈：** Rust 2024 版本，toml 0.8 crate 用于 TOML 解析，serde 用于序列化特性，自定义错误类型。

---

## 文件结构

基于 config.toml 和现有的 lib.rs：

**需要创建的文件：**

- `crates/config/src/input.rs` - InputNode 结构体和解析
- `crates/config/src/node.rs` - Node 枚举和 VirtualNode
- `crates/config/src/backend.rs` - BackendNode 和 BaseUrl
- `crates/config/tests/parser_tests.rs` - 所有 TDD 测试

**需要修改的文件：**

- `crates/config/src/lib.rs` - GatewayConfig 结构体（已存在，需要 FromStr 实现）
- `crates/config/src/error.rs` - Error 枚举变体
- `crates/config/Cargo.toml` - 添加 serde derive 特性

---

## TDD 流程说明

**重要：** 本计划采用严格的 TDD 流程：

1. **第一阶段：编写所有测试** - 先添加所有测试用例，全部运行失败
2. **第二阶段：实现功能** - 编写最小代码使所有测试通过

每个阶段完成后由人类手工提交，不自动执行 git 命令。

---

## 第一阶段：编写所有测试

---

### 任务 1：设置依赖和测试框架

**文件：**

- 创建：`crates/config/tests/parser_tests.rs`
- 修改：`crates/config/Cargo.toml`

- [ ] **步骤 1：添加 serde derive 到 Cargo.toml**

```toml
[package]
name = "llm-gateway-config"
version = "0.0.0"
edition = "2024"

[dependencies]
toml = "0.8"
serde = { version = "1.0", features = ["derive"] }
```

- [ ] **步骤 2：创建空测试文件**

```rust
// crates/config/tests/parser_tests.rs
use llm_gateway_config::GatewayConfig;
use std::str::FromStr;

// 所有测试将在后续任务中添加
```

- [ ] **步骤 3：验证测试框架编译通过**

```bash
cd crates/config && cargo test --no-run
```

预期：编译成功（无测试或测试失败）

---

### 任务 2：输入节点测试

**文件：**

- 修改：`crates/config/tests/parser_tests.rs`

- [ ] **步骤 1：添加输入节点测试**

```rust
#[test]
fn test_parse_input_node() {
    let toml_str = r#"
[input.service]
port = 8000
models = ["qwen3.5-35b-a3b", "kimi-k2.5"]
"#;
    let result = GatewayConfig::from_str(toml_str);
    assert!(result.is_ok());
    let config = result.unwrap();

    assert!(config.nodes.contains_key("service"));
    match &config.nodes["service"] {
        llm_gateway_config::Node::Input(input) => {
            assert_eq!(input.name.as_ref(), "service");
            assert_eq!(input.port, 8000);
            assert_eq!(input.models, vec!["qwen3.5-35b-a3b", "kimi-k2.5"]);
        }
        _ => panic!("Expected Input node"),
    }
}

#[test]
fn test_parse_input_node_multiple_models() {
    let toml_str = r#"
[input.service]
port = 8000
models = ["qwen3.5-35b-a3b", "qwen3.5-122b-a10b", "kimi-k2.5"]
"#;
    let result = GatewayConfig::from_str(toml_str);
    assert!(result.is_ok());
    let config = result.unwrap();

    match &config.nodes["service"] {
        llm_gateway_config::Node::Input(input) => {
            assert_eq!(input.models.len(), 3);
        }
        _ => panic!("Expected Input node"),
    }
}
```

- [ ] **步骤 2：运行测试验证失败**

```bash
cd crates/config && cargo test test_parse_input_node 2>&1 | head -20
```

预期：失败 - `from_str` 返回 `todo!()` panic 或类型未定义

---

### 任务 3：虚拟节点测试

**文件：**

- 修改：`crates/config/tests/parser_tests.rs`

- [ ] **步骤 1：添加虚拟节点序列路由测试**

```rust
#[test]
fn test_parse_virtual_node_sequence() {
    let toml_str = r#"
[node.qwen3.5-35b-a3b]
sequence = ["sglang-qwen3.5-35b-a3b", "aliyun"]
"#;
    let result = GatewayConfig::from_str(toml_str);
    assert!(result.is_ok());
    let config = result.unwrap();

    assert!(config.nodes.contains_key("qwen3.5-35b-a3b"));
    match &config.nodes["qwen3.5-35b-a3b"] {
        llm_gateway_config::Node::Virtual(virtual_node) => {
            assert_eq!(virtual_node.sequence(), &["sglang-qwen3.5-35b-a3b", "aliyun"]);
        }
        _ => panic!("Expected Virtual node"),
    }
}

#[test]
fn test_parse_multiple_virtual_nodes() {
    let toml_str = r#"
[node.model-a]
sequence = ["backend-1", "backend-2"]

[node.model-b]
sequence = ["backend-3", "backend-4"]
"#;
    let result = GatewayConfig::from_str(toml_str);
    assert!(result.is_ok());
    let config = result.unwrap();

    assert!(config.nodes.contains_key("model-a"));
    assert!(config.nodes.contains_key("model-b"));
}
```

- [ ] **步骤 2：运行测试验证失败**

```bash
cd crates/config && cargo test test_parse_virtual_node_sequence 2>&1 | head -20
```

预期：失败 - VirtualNode 未实现

---

### 任务 4：后端节点测试

**文件：**

- 修改：`crates/config/tests/parser_tests.rs`

- [ ] **步骤 1：添加后端节点测试**

```rust
#[test]
fn test_parse_simple_backend() {
    let toml_str = r#"
[backend.sglang-qwen3.5-35b-a3b]
base-url = "http://172.17.250.163:30001"
"#;
    let result = GatewayConfig::from_str(toml_str);
    assert!(result.is_ok());
    let config = result.unwrap();

    assert!(config.nodes.contains_key("sglang-qwen3.5-35b-a3b"));
    match &config.nodes["sglang-qwen3.5-35b-a3b"] {
        llm_gateway_config::Node::Backend(backend) => {
            assert_eq!(backend.base_url.default(), "http://172.17.250.163:30001");
            assert!(backend.api_key.is_none());
        }
        _ => panic!("Expected Backend node"),
    }
}

#[test]
fn test_parse_backend_with_protocol_specific_urls() {
    let toml_str = r#"
[backend.aliyun]
base-url = { anthropic = "https://dashscope.aliyuncs.com/apps/anthropic" }
api-key = "$ALIYUN_API_KEY"
"#;
    let result = GatewayConfig::from_str(toml_str);
    assert!(result.is_ok());
    let config = result.unwrap();

    assert!(config.nodes.contains_key("aliyun"));
    match &config.nodes["aliyun"] {
        llm_gateway_config::Node::Backend(backend) => {
            assert_eq!(
                backend.base_url.get("anthropic"),
                Some("https://dashscope.aliyuncs.com/apps/anthropic")
            );
            assert_eq!(backend.api_key, Some("$ALIYUN_API_KEY".to_string()));
        }
        _ => panic!("Expected Backend node"),
    }
}

#[test]
fn test_parse_backend_with_default_url() {
    let toml_str = r#"
[backend.aliyun]
base-url = { default = "https://default.url", anthropic = "https://anthropic.url" }
"#;
    let result = GatewayConfig::from_str(toml_str);
    assert!(result.is_ok());
    let config = result.unwrap();

    match &config.nodes["aliyun"] {
        llm_gateway_config::Node::Backend(backend) => {
            assert_eq!(backend.base_url.default(), "https://default.url");
            assert_eq!(backend.base_url.get("anthropic"), Some("https://anthropic.url"));
        }
        _ => panic!("Expected Backend node"),
    }
}
```

- [ ] **步骤 2：运行测试验证失败**

```bash
cd crates/config && cargo test test_parse_simple_backend 2>&1 | head -20
```

预期：失败 - BackendNode 未实现

---

### 任务 5：完整集成测试

**文件：**

- 修改：`crates/config/tests/parser_tests.rs`

- [ ] **步骤 1：添加完整 config.toml 集成测试**

```rust
#[test]
fn test_parse_full_config() {
    let toml_str = r#"
[input.service]
port = 8000
models = ["qwen3.5-35b-a3b", "qwen3.5-122b-a10b", "kimi-k2.5"]

[node.qwen3.5-35b-a3b]
sequence = ["sglang-qwen3.5-35b-a3b", "aliyun"]

[node.qwen3.5-122b-a10b]
sequence = ["sglang-qwen3.5-122b-a10b", "aliyun"]

[backend.sglang-qwen3.5-35b-a3b]
base-url = "http://172.17.250.163:30001"

[backend.sglang-qwen3.5-122b-a10b]
base-url = "http://172.17.250.163:30002"

[backend.sglang-kimi-k2.5]
base-url = "http://172.17.250.176:30001"

[backend.aliyun]
base-url = { anthropic = "https://dashscope.aliyuncs.com/apps/anthropic" }
api-key = "$ALIYUN_API_KEY"
"#;

    let result = GatewayConfig::from_str(toml_str);
    assert!(result.is_ok());
    let config = result.unwrap();

    // 验证输入节点
    assert!(config.nodes.contains_key("service"));
    match &config.nodes["service"] {
        llm_gateway_config::Node::Input(input) => {
            assert_eq!(input.port, 8000);
            assert_eq!(input.models.len(), 3);
        }
        _ => panic!("Expected Input node"),
    }

    // 验证虚拟节点
    assert!(config.nodes.contains_key("qwen3.5-35b-a3b"));
    assert!(config.nodes.contains_key("qwen3.5-122b-a10b"));

    // 验证后端
    assert!(config.nodes.contains_key("sglang-qwen3.5-35b-a3b"));
    assert!(config.nodes.contains_key("aliyun"));

    match &config.nodes["aliyun"] {
        llm_gateway_config::Node::Backend(backend) => {
            assert_eq!(backend.api_key, Some("$ALIYUN_API_KEY".to_string()));
        }
        _ => panic!("Expected Backend node"),
    }
}
```

- [ ] **步骤 2：运行测试验证失败**

```bash
cd crates/config && cargo test test_parse_full_config 2>&1 | head -20
```

预期：失败 - 功能未实现

---

### 任务 6：错误处理测试

**文件：**

- 修改：`crates/config/tests/parser_tests.rs`

- [ ] **步骤 1：添加错误处理测试**

```rust
#[test]
fn test_error_on_missing_port() {
    let toml_str = r#"
[input.service]
models = ["model1"]
"#;
    let result = GatewayConfig::from_str(toml_str);
    assert!(result.is_err());

    match result.unwrap_err() {
        llm_gateway_config::ConfigParseError::MissingField(field) => {
            assert_eq!(field, "port");
        }
        _ => panic!("Expected MissingField error"),
    }
}

#[test]
fn test_error_on_missing_base_url() {
    let toml_str = r#"
[backend.test]
api-key = "test-key"
"#;
    let result = GatewayConfig::from_str(toml_str);
    assert!(result.is_err());

    match result.unwrap_err() {
        llm_gateway_config::ConfigParseError::MissingField(field) => {
            assert_eq!(field, "base-url");
        }
        _ => panic!("Expected MissingField error"),
    }
}

#[test]
fn test_error_on_invalid_toml() {
    let toml_str = "invalid [ toml";
    let result = GatewayConfig::from_str(toml_str);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), llm_gateway_config::ConfigParseError::ParseError(_)));
}

#[test]
fn test_error_on_duplicate_input_name() {
    let toml_str = r#"
[input.service]
port = 8000
models = ["model1"]

[input.service]
port = 9000
models = ["model2"]
"#;
    let result = GatewayConfig::from_str(toml_str);
    assert!(result.is_err());

    match result.unwrap_err() {
        llm_gateway_config::ConfigParseError::DuplicateName(name) => {
            assert_eq!(name, "service");
        }
        _ => panic!("Expected DuplicateName error"),
    }
}

#[test]
fn test_error_on_duplicate_node_name() {
    let toml_str = r#"
[node.test]
sequence = ["a", "b"]

[node.test]
sequence = ["c", "d"]
"#;
    let result = GatewayConfig::from_str(toml_str);
    assert!(result.is_err());

    match result.unwrap_err() {
        llm_gateway_config::ConfigParseError::DuplicateName(name) => {
            assert_eq!(name, "test");
        }
        _ => panic!("Expected DuplicateName error"),
    }
}

#[test]
fn test_error_on_duplicate_backend_name() {
    let toml_str = r#"
[backend.test]
base-url = "http://test1.com"

[backend.test]
base-url = "http://test2.com"
"#;
    let result = GatewayConfig::from_str(toml_str);
    assert!(result.is_err());

    match result.unwrap_err() {
        llm_gateway_config::ConfigParseError::DuplicateName(name) => {
            assert_eq!(name, "test");
        }
        _ => panic!("Expected DuplicateName error"),
    }
}

#[test]
fn test_error_on_cross_type_duplicate_name() {
    let toml_str = r#"
[input.service]
port = 8000
models = ["model1"]

[backend.service]
base-url = "http://test.com"
"#;
    let result = GatewayConfig::from_str(toml_str);
    assert!(result.is_err());

    match result.unwrap_err() {
        llm_gateway_config::ConfigParseError::DuplicateName(name) => {
            assert_eq!(name, "service");
        }
        _ => panic!("Expected DuplicateName error"),
    }
}
```

- [ ] **步骤 2：运行测试验证失败**

```bash
cd crates/config && cargo test test_error_on_missing_port 2>&1 | head -20
```

预期：失败 - 错误类型未实现

---

### 任务 7：第一阶段完成验证

**文件：**

- 无

- [ ] **步骤 1：运行所有测试确认全部失败**

```bash
cd crates/config && cargo test 2>&1 | head -50
```

预期：所有测试失败（编译错误或运行时失败）

---

## 第二阶段：实现功能

---

### 任务 8：实现错误类型

**文件：**

- 修改：`crates/config/src/error.rs`

- [ ] **步骤 1：实现错误枚举**

```rust
// crates/config/src/error.rs
use std::{error, fmt};

#[derive(Clone, Debug)]
pub enum Error {
    ParseError(String),
    MissingField(String),
    DuplicateName(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::ParseError(msg) => write!(f, "解析错误：{}", msg),
            Error::MissingField(field) => write!(f, "缺少必需字段：{}", field),
            Error::DuplicateName(name) => write!(f, "重复的节点名称：{}", name),
        }
    }
}

impl error::Error for Error {}
```

---

### 任务 9：实现输入节点模块

**文件：**

- 创建：`crates/config/src/input.rs`
- 修改：`crates/config/src/lib.rs`

- [ ] **步骤 1：创建 InputNode 结构体**

```rust
// crates/config/src/input.rs
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct InputNode {
    pub name: Arc<str>,
    pub port: u16,
    pub models: Vec<String>,
}
```

- [ ] **步骤 2：更新 lib.rs 导入模块**

```rust
// crates/config/src/lib.rs
mod error;
mod input;

pub use error::Error as ConfigParseError;
pub use input::InputNode;
use std::{collections::HashMap, str::FromStr, sync::Arc};
```

---

### 任务 10：实现虚拟节点模块

**文件：**

- 创建：`crates/config/src/node.rs`
- 修改：`crates/config/src/lib.rs`

- [ ] **步骤 1：创建 VirtualNode 枚举**

```rust
// crates/config/src/node.rs
#[derive(Clone, Debug)]
pub enum VirtualNode {
    Sequence(Vec<String>),
}

impl VirtualNode {
    pub fn sequence(&self) -> &[String] {
        match self {
            VirtualNode::Sequence(seq) => seq,
        }
    }
}
```

- [ ] **步骤 2：更新 lib.rs 导入模块并添加 Node 枚举**

```rust
// crates/config/src/lib.rs
mod error;
mod input;
mod node;

pub use error::Error as ConfigParseError;
pub use input::InputNode;
pub use node::{Node, VirtualNode};
use std::{collections::HashMap, str::FromStr, sync::Arc};

#[derive(Clone, Debug)]
pub struct GatewayConfig {
    pub nodes: HashMap<Arc<str>, Node>,
}
```

---

### 任务 11：实现后端节点模块

**文件：**

- 创建：`crates/config/src/backend.rs`
- 修改：`crates/config/src/lib.rs`

- [ ] **步骤 1：创建 BackendNode 和 BaseUrl 结构体**

```rust
// crates/config/src/backend.rs
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct BackendNode {
    pub base_url: BaseUrl,
    pub api_key: Option<String>,
}

#[derive(Clone, Debug)]
pub struct BaseUrl {
    pub map: HashMap<String, String>,
    pub default: String,
}

impl BaseUrl {
    pub fn default(&self) -> &str {
        &self.default
    }

    pub fn get(&self, protocol: &str) -> Option<&str> {
        self.map.get(protocol).map(|s| s.as_str())
    }
}
```

- [ ] **步骤 2：更新 lib.rs 导入模块并添加 Backend 变体**

```rust
// crates/config/src/lib.rs
mod error;
mod input;
mod node;
mod backend;

pub use error::Error as ConfigParseError;
pub use input::InputNode;
pub use node::{Node, VirtualNode};
pub use backend::{BackendNode, BaseUrl};
use std::{collections::HashMap, str::FromStr, sync::Arc};

#[derive(Clone, Debug)]
pub struct GatewayConfig {
    pub nodes: HashMap<Arc<str>, Node>,
}

#[derive(Clone, Debug)]
pub enum Node {
    Input(InputNode),
    Virtual(VirtualNode),
    Backend(BackendNode),
}
```

---

### 任务 12：实现 FromStr 解析逻辑

**文件：**

- 修改：`crates/config/src/lib.rs`

- [ ] **步骤 1：实现完整的 from_str 方法**

```rust
impl FromStr for GatewayConfig {
    type Err = ConfigParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parsed = toml::from_str::<toml::Table>(s)
            .map_err(|e| ConfigParseError::ParseError(e.to_string()))?;

        let mut nodes = HashMap::new();

        // 解析输入节点 [input.*]
        if let Some(input_table) = parsed.get("input").and_then(|v| v.as_table()) {
            for (name, value) in input_table {
                if nodes.contains_key(name.as_str()) {
                    return Err(ConfigParseError::DuplicateName(name.clone()));
                }

                if let Some(node_table) = value.as_table() {
                    let port = node_table
                        .get("port")
                        .and_then(|v| v.as_integer())
                        .ok_or_else(|| ConfigParseError::MissingField("port".to_string()))? as u16;

                    let models = node_table
                        .get("models")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default();

                    nodes.insert(
                        name.clone().into(),
                        Node::Input(InputNode {
                            name: name.clone().into(),
                            port,
                            models,
                        }),
                    );
                }
            }
        }

        // 解析虚拟节点 [node.*]
        if let Some(node_table) = parsed.get("node").and_then(|v| v.as_table()) {
            for (name, value) in node_table {
                if nodes.contains_key(name.as_str()) {
                    return Err(ConfigParseError::DuplicateName(name.clone()));
                }

                if let Some(node_config) = value.as_table() {
                    if let Some(sequence) = node_config
                        .get("sequence")
                        .and_then(|v| v.as_array())
                    {
                        let seq = sequence
                            .iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect();

                        nodes.insert(
                            name.clone().into(),
                            Node::Virtual(VirtualNode::Sequence(seq)),
                        );
                    }
                }
            }
        }

        // 解析后端节点 [backend.*]
        if let Some(backend_table) = parsed.get("backend").and_then(|v| v.as_table()) {
            for (name, value) in backend_table {
                if nodes.contains_key(name.as_str()) {
                    return Err(ConfigParseError::DuplicateName(name.clone()));
                }

                if let Some(backend_config) = value.as_table() {
                    let base_url = if let Some(url_value) = backend_config.get("base-url") {
                        if let Some(url_str) = url_value.as_str() {
                            // 简单字符串格式
                            BaseUrl {
                                map: HashMap::new(),
                                default: url_str.to_string(),
                            }
                        } else if let Some(url_table) = url_value.as_table() {
                            // 表格式带协议特定 URL
                            let mut map = HashMap::new();
                            let mut default = String::new();
                            for (protocol, url) in url_table {
                                if let Some(url_str) = url.as_str() {
                                    if protocol == "default" {
                                        default = url_str.to_string();
                                    } else {
                                        map.insert(protocol.clone(), url_str.to_string());
                                    }
                                }
                            }
                            if default.is_empty() {
                                default = map.values().next().cloned().unwrap_or_default();
                            }
                            BaseUrl { map, default }
                        } else {
                            return Err(ConfigParseError::ParseError(
                                "base-url 必须是字符串或表".to_string(),
                            ));
                        }
                    } else {
                        return Err(ConfigParseError::MissingField("base-url".to_string()));
                    };

                    let api_key = backend_config
                        .get("api-key")
                        .and_then(|v| v.as_str())
                        .map(String::from);

                    nodes.insert(
                        name.clone().into(),
                        Node::Backend(BackendNode { base_url, api_key }),
                    );
                }
            }
        }

        Ok(GatewayConfig { nodes })
    }
}
```

---

### 任务 13：验证所有测试通过

**文件：**

- 无

- [ ] **步骤 1：运行所有测试**

```bash
cd crates/config && cargo test
```

预期：所有测试通过

- [ ] **步骤 2：运行 clippy 检查代码质量**

```bash
cd crates/config && cargo clippy -- -D warnings
```

预期：无警告

---

## 计划审查

计划完成。准备执行。

**执行选项：**

1. **子代理驱动（推荐）** - 每个任务分派新的子代理，任务间审查
2. **内联执行** - 在当前会话中执行任务，设置检查点

**选择哪种方式？**
