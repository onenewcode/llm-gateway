/// 管理 API 配置
#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct AdminConfig {
    /// 管理 API 端口（0 表示随机端口）
    pub port: u16,
    /// 认证令牌（None 表示无需认证）
    #[serde(rename = "auth-token")]
    pub auth_token: Option<String>,
}
