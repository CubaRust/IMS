//! identity 视图 DTO

use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserView {
    pub id: i64,
    pub user_code: String,
    pub user_name: String,
    pub login_name: String,
    pub mobile: Option<String>,
    pub is_active: bool,
    pub roles: Vec<String>,
    pub created_at: PrimitiveDateTime,
    pub updated_at: PrimitiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleView {
    pub id: i64,
    pub role_code: String,
    pub role_name: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionView {
    pub id: i64,
    pub perm_code: String,
    pub perm_name: String,
    pub module_code: String,
    pub action_code: String,
}
