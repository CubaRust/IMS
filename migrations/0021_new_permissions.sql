-- ============================================================================
-- 0021_new_permissions.sql
-- 补全所有 require_permission 引用但 seed 中缺失的权限点,并分配给 ADMIN
-- ============================================================================

-- 1. 插入缺失的权限点
insert into sys.sys_permission (perm_code, perm_name, module_code, action_code) values
    -- defect 补 submit/void
    ('defect.submit',              '不良提交',         'quality',    'submit'),
    ('defect.void',                '不良作废',         'quality',    'void'),
    -- scrap 补 view/submit/void
    ('scrap.view',                 '报废查看',         'quality',    'view'),
    ('scrap.submit',               '报废提交',         'quality',    'submit'),
    ('scrap.void',                 '报废作废',         'quality',    'void'),
    -- recovery 补 view/submit/void
    ('recovery.view',              '拆解查看',         'quality',    'view'),
    ('recovery.submit',            '拆解提交',         'quality',    'submit'),
    ('recovery.void',              '拆解作废',         'quality',    'void'),
    -- customer_return 补 create/submit/void
    ('customer_return.create',     '客退录入',         'return',     'create'),
    ('customer_return.submit',     '客退提交',         'return',     'submit'),
    ('customer_return.void',       '客退作废',         'return',     'void'),
    -- supplier_return 补 submit/void
    ('supplier_return.submit',     '退供提交',         'return',     'submit'),
    ('supplier_return.void',       '退供作废',         'return',     'void'),
    -- pmc.outsource.* (seed 里是 outsource.*, 代码用 pmc.outsource.*)
    ('pmc.outsource.view',         '委外查看',         'outsource',  'view'),
    ('pmc.outsource.create',       '委外录入',         'outsource',  'create'),
    ('pmc.outsource.send',         '委外发料',         'outsource',  'send'),
    ('pmc.outsource.back',         '委外回料',         'outsource',  'back'),
    ('pmc.outsource.void',         '委外作废',         'outsource',  'void'),
    -- stocktake 补 count/submit/void
    ('stocktake.count',            '盘点录数',         'stocktake',  'count'),
    ('stocktake.submit',           '盘点提交',         'stocktake',  'submit'),
    ('stocktake.void',             '盘点作废',         'stocktake',  'void'),
    -- report
    ('report.view',                '报表查看',         'report',     'view'),
    -- system config
    ('sys.dict.view',              '数据字典查看',     'sys',        'view'),
    ('sys.dict.edit',              '数据字典编辑',     'sys',        'edit'),
    ('sys.doc_no_rule.view',       '编码规则查看',     'sys',        'view'),
    ('sys.doc_no_rule.edit',       '编码规则编辑',     'sys',        'edit'),
    -- recovery template
    ('mdm.recovery_tpl.view',      '回收模板查看',     'mdm',        'view'),
    ('mdm.recovery_tpl.edit',      '回收模板编辑',     'mdm',        'edit')
on conflict (perm_code) do nothing;

-- 2. 把所有新权限授予 ADMIN(ADMIN 拥有全部权限)
insert into sys.sys_role_permission (role_id, permission_id)
select r.id, p.id
  from sys.sys_role r
  cross join sys.sys_permission p
 where r.role_code = 'ADMIN'
on conflict (role_id, permission_id) do nothing;
