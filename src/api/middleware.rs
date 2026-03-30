//! 认证中间件

use http::header::AUTHORIZATION;

/// 认证中间件
pub struct AuthMiddleware {
    token: Option<String>,
}

impl AuthMiddleware {
    pub fn new(token: Option<String>) -> Self {
        Self { token }
    }

    pub fn authenticate(&self, headers: &http::HeaderMap) -> bool {
        // 未配置令牌时允许所有请求
        let Some(expected) = &self.token else {
            return true;
        };

        // 检查 Bearer 令牌
        if let Some(auth_value) = headers.get(AUTHORIZATION)
            && let Ok(auth_str) = auth_value.to_str()
            && let Some(token) = auth_str.strip_prefix("Bearer ")
        {
            return token == expected;
        }

        false
    }
}
