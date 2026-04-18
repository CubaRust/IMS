-- ============================================================================
-- 0017_jwt_revocation.sql
-- JWT 吊销黑名单(登出/强制下线)
--
-- 策略:存 jti(JWT ID)+ 过期时间。auth_guard 校验时查 jti 是否在黑名单。
-- 过期后 jti 可被定时任务清理(本期不做清理作业,手工 delete)。
-- ============================================================================

create table if not exists sys.sys_jwt_revocation (
    jti         text primary key,
    user_id     bigint,
    login_name  text,
    reason      text not null default 'LOGOUT',   -- LOGOUT / FORCE_LOGOUT / PASSWORD_CHANGE
    revoked_at  timestamp not null default now(),
    expires_at  timestamp not null                -- 过期后可清理
);

create index if not exists idx_jwt_rev_user
    on sys.sys_jwt_revocation(user_id)
    where user_id is not null;

create index if not exists idx_jwt_rev_expires
    on sys.sys_jwt_revocation(expires_at);

comment on table sys.sys_jwt_revocation is 'JWT 吊销黑名单';
comment on column sys.sys_jwt_revocation.jti is 'JWT ID(uuid)';
comment on column sys.sys_jwt_revocation.reason is '吊销原因:LOGOUT / FORCE_LOGOUT / PASSWORD_CHANGE';
