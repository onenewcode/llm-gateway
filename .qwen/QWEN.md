# 项目备忘录 - RustLLM Gateway

## Markdown 写作规范

1. 遵循 markdownlint 规则
2. 注意 MD060 表格列样式（表格列对齐）
3. 使用严格等宽字体排版，中文（全角）字符显示宽度为 2 个英文（半角）字符，Markdown 表格按此标准对齐
4. 不要在标题之前加 --- 分隔线

## Rust struct 可见性设计原则

1. **组织数据的 struct（POD 类型）** - 所有字段都应为 `pub`
   - 例如：`RoutingPayload`, `NodeMetrics`
   - 作为数据载体，不包含复杂逻辑

2. **实现功能的 struct** - 所有字段都应为非 `pub`
   - 例如：`InputNode`, `VirtualNode`, `OutputNode`, `RoutingGraph`
   - 包含复杂逻辑或对字段状态有要求的方法
   - 通过方法访问，避免外部代码意外修改字段状态导致异常

## Git 提交规范

**重要：** 在任何模式下（包括 YOLO），每次提交 git 之前必须由人类 review

- 提交前执行 `git status` 和 `git diff --staged` 查看变更
- 等待人类确认提交信息和变更内容
- 获得批准后再执行 `git commit`

## TDD 开发流程

对于每个功能模块：

```text
1. 文档准备（协议调研/技术调研）
       ↓
2. 设计接口（定义 Trait/Struct）
       ↓
3. 编写测试（此时编译失败或测试失败）
       ↓
4. 实现功能（测试通过）
       ↓
5. 重构优化
```

## 项目结构

```text
rustllm-gateway/
├── Cargo.toml              # Workspace 根
├── crates/
│   ├── gateway-protocol/   # 协议定义
│   ├── gateway-core/       # 核心路由逻辑
│   ├── gateway-adapters/   # 后端适配器
│   ├── gateway-metrics/    # 指标收集
│   ├── gateway-config/     # 配置管理
│   └── gateway-cli/        # CLI 工具
├── src/                    # 主程序（二进制）
├── docs/
│   ├── design/             # 设计文档
│   ├── protocols/          # 协议文档
│   ├── plan/               # 计划文档
│   └── research/           # 调研文档
└── tests/                  # 集成测试
```

## 核心设计决策

1. **Input Node = HTTP Server** - 每个输入节点是一个独立的 HTTP 服务，监听独立端口
2. **协议动态识别** - Input Node 不预设协议，根据请求 path/body 动态识别
3. **RoutingPayload = 已解析 HTTP 包 + 节点追溯** - 携带经过的节点表，支持回溯和调试
4. **健康感知内置** - 所有节点内置健康检查，DFS 路由 + 回溯机制
5. **图结构纯粹** - 节点直接引用 `Arc<Node>`，无需 Edge 类型
6. **策略封装权重** - 权重等信息在路由策略内部，不在图结构中

## 开发阶段

- **Phase 1 (MVP)**: 1.5 周 - 核心代理、Tier 1 协议、健康感知 DFS、基础统计
- **Phase 2**: 1.5 周 - 高级负载均衡、非流式热迁移、Tier 2 协议、CLI 增强
- **Phase 3**: 1.5 周 - 限流、HTTP API、JWT 认证、分布式追踪
- **Phase 4**: 2 周 - 插件系统、语义缓存、多区域集群、流式热迁移

**总计**: 6.5 周完成全部开发

## 协议文档

- `docs/protocols/openai.md` - OpenAI Chat Completion API 完整协议
- `docs/protocols/anthropic.md` - Anthropic Messages API 完整协议

## 技术栈

- **Async 运行时**: tokio 1.x
- **HTTP 框架**: hyper 1.x (不使用 axum)
- **序列化**: serde 1.x + serde_json
- **配置**: toml 0.8
- **错误处理**: thiserror 2.x
- **日志**: tracing 0.1
- **并发容器**: dashmap 5.x
- **指标**: prometheus 0.13
- **CLI**: clap 4.x
