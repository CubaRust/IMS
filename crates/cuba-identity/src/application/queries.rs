//! identity 查询

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct QueryUsers {
    pub login_name: Option<String>,
    pub user_name: Option<String>,
    pub is_active: Option<bool>,
    pub role_code: Option<String>,
    /// 租户 id(service 层注入)
    #[serde(skip)]
    pub tenant_id: Option<i64>,
}
