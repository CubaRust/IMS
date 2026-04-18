-- ============================================================================
-- 0021_new_permissions.sql
-- 新增权限点并分配给 ADMIN 角色
-- ============================================================================

-- 1. 插入新权限点
insert into sys.sys_permission (permission_code, permission_name, module, remark) values
    ('sys.dict.view',           '数据字典-查看',     'sys',  '查看数据字典'),
    ('sys.dict.edit',           '数据字典-编辑',     'sys',  '新增/修改数据字典'),
    ('sys.doc_no_rule.view',    '编码规则-查看',     'sys',  '查看单据编码规则'),
    ('sys.doc_no_rule.edit',    '编码规则-编辑',     'sys',  '修改单据编码规则'),
    ('mdm.recovery_tpl.view',   '回收模板-查看',     'mdm',  '查看回收拆解模板'),
    ('mdm.recovery_tpl.edit',   '回收模板-编辑',     'mdm',  '新增/修改回收拆解模板')
on conflict (permission_code) do nothing;

-- 2. 分配给 ADMIN 角色
insert into sys.sys_role_permission (role_id, permission_id)
select r.id, p.id
  from sys.sys_role r
  cross join sys.sys_permission p
 where r.role_code = 'ADMIN'
   and p.permission_code in (
       'sys.dict.view', 'sys.dict.edit',
       'sys.doc_no_rule.view', 'sys.doc_no_rule.edit',
       'mdm.recovery_tpl.view', 'mdm.recovery_tpl.edit'
   )
on conflict do nothing;
