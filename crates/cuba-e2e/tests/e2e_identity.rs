//! identity e2e:登录、改密、用户列表

use cuba_identity::{ChangePasswordCommand, IdentityService, LoginCommand};
use cuba_testkit::{fixtures::admin_ctx, TestDb};

const JWT_SECRET: &str = "test-secret";
const JWT_TTL: i64 = 3600;

#[tokio::test]
async fn admin_login_default_password_works() {
    let db = TestDb::new().await;
    let svc = IdentityService::new(db.pool_owned(), JWT_SECRET, JWT_TTL);

    let result = svc
        .login(LoginCommand {
            login_name: "admin".into(),
            password: "Admin@123".into(),
        })
        .await
        .expect("admin login");

    assert!(!result.token.is_empty());
    assert_eq!(result.login_name, "admin");
    assert!(result.permissions.len() > 0, "admin should have permissions");
}

#[tokio::test]
async fn login_wrong_password_returns_error() {
    let db = TestDb::new().await;
    let svc = IdentityService::new(db.pool_owned(), JWT_SECRET, JWT_TTL);

    let err = svc
        .login(LoginCommand {
            login_name: "admin".into(),
            password: "WrongPass123!".into(),
        })
        .await
        .expect_err("wrong password should fail");
    let dbg = format!("{err:?}");
    // 错误码段 110xx 或 401
    assert!(
        dbg.contains("11") || dbg.contains("401") || dbg.contains("UNAUTH"),
        "should be auth error: {dbg}"
    );
}

#[tokio::test]
async fn change_password_then_login_with_new() {
    let db = TestDb::new().await;
    let svc = IdentityService::new(db.pool_owned(), JWT_SECRET, JWT_TTL);

    // 先登录拿到 ctx
    let _login = svc
        .login(LoginCommand {
            login_name: "admin".into(),
            password: "Admin@123".into(),
        })
        .await
        .unwrap();

    // 改密
    let ctx = admin_ctx();
    svc.change_password(
        &ctx,
        ChangePasswordCommand {
            old_password: "Admin@123".into(),
            new_password: "NewPass@456".into(),
        },
    )
    .await
    .expect("change password");

    // 新密码登得上
    let ok = svc
        .login(LoginCommand {
            login_name: "admin".into(),
            password: "NewPass@456".into(),
        })
        .await
        .expect("login with new password");
    assert!(!ok.token.is_empty());

    // 旧密码登不上
    let fail = svc
        .login(LoginCommand {
            login_name: "admin".into(),
            password: "Admin@123".into(),
        })
        .await;
    assert!(fail.is_err(), "old password should fail after change");
}

#[tokio::test]
async fn list_users_returns_at_least_admin() {
    let db = TestDb::new().await;
    let svc = IdentityService::new(db.pool_owned(), JWT_SECRET, JWT_TTL);

    let users = svc
        .list_users(&admin_ctx(), &cuba_identity::application::QueryUsers::default())
        .await
        .expect("list users");

    assert!(users.iter().any(|u| u.login_name == "admin"));
}
