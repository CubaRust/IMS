//! 配置加载
//!
//! 优先级:env > .env 文件 > 默认值
//! 本期不引入 YAML/TOML 配置文件,完全走环境变量,简化部署。

use serde::Deserialize;

use cuba_shared::error::AppError;

/// 运行环境
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppEnv {
    Dev,
    Test,
    Staging,
    Prod,
}

impl AppEnv {
    #[must_use]
    pub const fn is_prod(self) -> bool {
        matches!(self, Self::Prod)
    }
}

impl std::str::FromStr for AppEnv {
    type Err = AppError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_ascii_lowercase().as_str() {
            "dev" | "development" => Self::Dev,
            "test" => Self::Test,
            "staging" | "stage" | "pre" => Self::Staging,
            "prod" | "production" => Self::Prod,
            other => {
                return Err(AppError::validation(format!("未知的 APP_ENV: {other}")));
            }
        })
    }
}

/// Migration 模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationMode {
    /// 启动时自动跑 sqlx::migrate!()
    Auto,
    /// 启动时仅校验,有未应用 migration 则拒启动
    Manual,
}

impl std::str::FromStr for MigrationMode {
    type Err = AppError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_ascii_lowercase().as_str() {
            "auto" => Self::Auto,
            "manual" => Self::Manual,
            other => {
                return Err(AppError::validation(format!(
                    "未知的 MIGRATION_MODE: {other}"
                )));
            }
        })
    }
}

/// 应用配置
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub app_name: String,
    pub app_env: AppEnv,
    pub http_host: String,
    pub http_port: u16,
    pub database_url: String,
    /// 读副本 URL;None 时读走主库
    pub database_read_url: Option<String>,
    pub database_max_connections: u32,
    pub jwt_secret: String,
    pub jwt_ttl_seconds: i64,
    pub migration_mode: MigrationMode,
}

impl AppConfig {
    /// 从进程环境变量加载,先尝试加载 `.env`
    pub fn from_env() -> Result<Self, AppError> {
        // .env 是可选的:prod 通常不放 .env,直接给环境变量
        let _ = dotenvy::dotenv();

        let app_env: AppEnv = read_env("APP_ENV", "dev")?.parse()?;
        let migration_mode: MigrationMode = read_env("MIGRATION_MODE", "auto")?.parse()?;

        // prod 模式禁止 MIGRATION_MODE=auto,避免生产误跑
        if app_env.is_prod() && migration_mode == MigrationMode::Auto {
            return Err(AppError::validation(
                "生产环境禁止 MIGRATION_MODE=auto,请改为 manual 并在发布流水线独立执行迁移",
            ));
        }

        let jwt_secret = read_env("JWT_SECRET", "")?;
        if jwt_secret.is_empty() {
            return Err(AppError::validation("JWT_SECRET 未设置"));
        }
        if app_env.is_prod() && jwt_secret.len() < 32 {
            return Err(AppError::validation("生产环境 JWT_SECRET 长度至少 32 字符"));
        }

        let cfg = Self {
            app_name: read_env("APP_NAME", "EQYCC_CUBA_IMS")?,
            app_env,
            http_host: read_env("HTTP_HOST", "0.0.0.0")?,
            http_port: read_env_parse("HTTP_PORT", 8080)?,
            database_url: read_env("DATABASE_URL", "")?,
            database_read_url: {
                let v = read_env("DATABASE_READ_URL", "")?;
                if v.is_empty() {
                    None
                } else {
                    Some(v)
                }
            },
            database_max_connections: read_env_parse("DATABASE_MAX_CONNECTIONS", 10)?,
            jwt_secret,
            jwt_ttl_seconds: read_env_parse("JWT_TTL_SECONDS", 86400)?,
            migration_mode,
        };

        if cfg.database_url.is_empty() {
            return Err(AppError::validation("DATABASE_URL 未设置"));
        }
        Ok(cfg)
    }

    #[must_use]
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.http_host, self.http_port)
    }
}

// -- helpers -----------------------------------------------------------------

fn read_env(key: &str, default: &str) -> Result<String, AppError> {
    Ok(std::env::var(key).unwrap_or_else(|_| default.to_string()))
}

fn read_env_parse<T>(key: &str, default: T) -> Result<T, AppError>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match std::env::var(key) {
        Ok(v) => v
            .parse::<T>()
            .map_err(|e| AppError::validation(format!("{key} 解析失败: {e}"))),
        Err(_) => Ok(default),
    }
}
