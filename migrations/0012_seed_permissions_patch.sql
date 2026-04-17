-- ============================================================================
-- 0012_seed_permissions_patch.sql
-- 补 seed 里漏掉的细分权限点
--
-- 原 0010 里只给了 inv.balance.view / inv.txn.view,缺少事务提交权限。
-- cuba-inventory 路由层用 inv.txn.commit / inv.txn.view / inv.balance.view 三枚权限。
-- ============================================================================

insert into sys.sys_permission (perm_code, perm_name, module_code, action_code) values
    ('inv.txn.commit', '库存事务提交', 'inv', 'commit')
on conflict (perm_code) do nothing;

-- 把新权限授予 ADMIN
insert into sys.sys_role_permission (role_id, permission_id)
select r.id, p.id
  from sys.sys_role r
  cross join sys.sys_permission p
 where r.role_code = 'ADMIN'
   and p.perm_code = 'inv.txn.commit'
on conflict (role_id, permission_id) do nothing;

-- 同时授予仓管主管和仓管员(他们日常要收发货)
insert into sys.sys_role_permission (role_id, permission_id)
select r.id, p.id
  from sys.sys_role r
  cross join sys.sys_permission p
 where r.role_code in ('WH_MANAGER', 'WH_OPERATOR')
   and p.perm_code in ('inv.balance.view', 'inv.txn.view', 'inv.txn.commit')
on conflict (role_id, permission_id) do nothing;
