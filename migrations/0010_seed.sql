-- ============================================================================
-- 0010_seed.sql
-- 初始化种子数据(所有环境必跑)
--
-- 约定:
--   - 所有 insert 都用 on conflict do nothing,可重复执行
--   - 默认 admin 密码:Admin@123(bcrypt 哈希,实际上线请改)
-- ============================================================================

-- ---------------------------------------------------------------------------
-- 1. 单据编码规则
-- ---------------------------------------------------------------------------
insert into sys.sys_doc_no_rule (doc_type, doc_prefix, date_pattern, seq_length) values
    ('INBOUND',          'INB',  'YYYYMMDD', 4),
    ('OUTBOUND',         'OUT',  'YYYYMMDD', 4),
    ('INVENTORY_TXN',    'TXN',  'YYYYMMDD', 6),
    ('PREISSUE',         'PRE',  'YYYYMMDD', 4),
    ('DEFECT',           'DEF',  'YYYYMMDD', 4),
    ('RECOVERY',         'REC',  'YYYYMMDD', 4),
    ('SCRAP',            'SCR',  'YYYYMMDD', 4),
    ('CUSTOMER_RETURN',  'CRT',  'YYYYMMDD', 4),
    ('SUPPLIER_RETURN',  'SRT',  'YYYYMMDD', 4),
    ('OUTSOURCE',        'OSC',  'YYYYMMDD', 4),
    ('STOCKTAKE',        'STK',  'YYYYMMDD', 4)
on conflict (doc_type) do nothing;

-- ---------------------------------------------------------------------------
-- 2. 数据字典
-- ---------------------------------------------------------------------------
insert into sys.sys_dict (dict_type, dict_key, dict_value, dict_order) values
    ('MATERIAL_CATEGORY', 'RAW',       '原材料',    1),
    ('MATERIAL_CATEGORY', 'SEMI',      '半成品',    2),
    ('MATERIAL_CATEGORY', 'FINISHED',  '成品/总成', 3),
    ('MATERIAL_CATEGORY', 'PUBLIC',    '公共物料',  4),
    ('MATERIAL_CATEGORY', 'RECOVERY',  '回收件',    5),
    ('MATERIAL_CATEGORY', 'SCRAP',     '报废品',    6)
on conflict (dict_type, dict_key) do nothing;

insert into sys.sys_dict (dict_type, dict_key, dict_value, dict_order) values
    ('PROCESS_TYPE', 'GG',    'G+G',   1),
    ('PROCESS_TYPE', 'GF',    'G+F',   2),
    ('PROCESS_TYPE', 'TP',    '单TP',  3),
    ('PROCESS_TYPE', 'FOG',   'FOG',   4),
    ('PROCESS_TYPE', 'ASM',   '总成',  5),
    ('PROCESS_TYPE', 'OTHER', '其他',  99)
on conflict (dict_type, dict_key) do nothing;

insert into sys.sys_dict (dict_type, dict_key, dict_value, dict_order) values
    ('WH_TYPE', 'RAW_WH',     '原材料仓', 1),
    ('WH_TYPE', 'SEMI_WH',    '半成品仓', 2),
    ('WH_TYPE', 'FG_WH',      '成品仓',   3),
    ('WH_TYPE', 'BAD_WH',     '不良仓',   4),
    ('WH_TYPE', 'SCRAP_WH',   '报废仓',   5),
    ('WH_TYPE', 'TRANSIT_WH', '在途仓',   6),
    ('WH_TYPE', 'RETURN_WH',  '退货仓',   7),
    ('WH_TYPE', 'CHECK_WH',   '待检仓',   8)
on conflict (dict_type, dict_key) do nothing;

insert into sys.sys_dict (dict_type, dict_key, dict_value, dict_order) values
    ('STOCK_STATUS', 'TO_CHECK',                '待检',         1),
    ('STOCK_STATUS', 'QUALIFIED',               '合格',         2),
    ('STOCK_STATUS', 'BAD',                     '不良',         3),
    ('STOCK_STATUS', 'FROZEN',                  '冻结',         4),
    ('STOCK_STATUS', 'IN_PROCESS',              '在制',         5),
    ('STOCK_STATUS', 'OUTSOURCE',               '委外在途',     6),
    ('STOCK_STATUS', 'PREISSUE_PENDING',        '异常先发占用', 7),
    ('STOCK_STATUS', 'CUSTOMER_RETURN_PENDING', '客退待判定',   8),
    ('STOCK_STATUS', 'SCRAPPED',                '已报废',       9),
    ('STOCK_STATUS', 'RECOVERY',                '可回收',      10)
on conflict (dict_type, dict_key) do nothing;

insert into sys.sys_dict (dict_type, dict_key, dict_value, dict_order) values
    ('DOC_STATUS', 'DRAFT',     '草稿',   1),
    ('DOC_STATUS', 'SUBMITTED', '已提交', 2),
    ('DOC_STATUS', 'COMPLETED', '已完成', 3),
    ('DOC_STATUS', 'VOIDED',    '已作废', 4)
on conflict (dict_type, dict_key) do nothing;

insert into sys.sys_dict (dict_type, dict_key, dict_value, dict_order) values
    ('TXN_TYPE', 'IN',       '入',     1),
    ('TXN_TYPE', 'OUT',      '出',     2),
    ('TXN_TYPE', 'TRANSFER', '转',     3),
    ('TXN_TYPE', 'CONVERT',  '换',     4),
    ('TXN_TYPE', 'RESERVE',  '占用',   5),
    ('TXN_TYPE', 'RELEASE',  '释放',   6)
on conflict (dict_type, dict_key) do nothing;

-- ---------------------------------------------------------------------------
-- 3. 库存状态流转规则
-- ---------------------------------------------------------------------------
insert into mdm.mdm_status_flow (source_status, target_status, scene_code, need_auth_flag, remark) values
    ('TO_CHECK',  'QUALIFIED', 'IQC_PASS',    false, '来料检验合格'),
    ('TO_CHECK',  'BAD',       'IQC_FAIL',    false, '来料检验不合格'),
    ('TO_CHECK',  'FROZEN',    'IQC_HOLD',    true,  '来料暂时冻结'),
    ('QUALIFIED', 'IN_PROCESS', 'ISSUE_TO_PROD', false, '发料到生产'),
    ('QUALIFIED', 'OUTSOURCE',  'ISSUE_OUTSOURCE', false, '发给委外'),
    ('QUALIFIED', 'FROZEN',     'MANUAL_FREEZE', true, '手工冻结'),
    ('FROZEN',    'QUALIFIED',  'MANUAL_UNFREEZE', true, '解冻'),
    ('QUALIFIED', 'BAD',        'PROD_DEFECT', false, '生产发现不良'),
    ('BAD',       'RECOVERY',   'DISMANTLE', false, '不良拆解回收'),
    ('BAD',       'SCRAPPED',   'TO_SCRAP', true, '不良报废'),
    ('OUTSOURCE', 'QUALIFIED',  'OUTSOURCE_BACK_OK', false, '委外回料合格'),
    ('OUTSOURCE', 'BAD',        'OUTSOURCE_BACK_BAD', false, '委外回料不良'),
    ('CUSTOMER_RETURN_PENDING', 'QUALIFIED',        'CR_JUDGE_OK',   true, '客退判定合格'),
    ('CUSTOMER_RETURN_PENDING', 'BAD',              'CR_JUDGE_BAD',  false, '客退判定不良'),
    ('CUSTOMER_RETURN_PENDING', 'RECOVERY',         'CR_JUDGE_DISMANTLE', false, '客退待拆解'),
    ('CUSTOMER_RETURN_PENDING', 'SCRAPPED',         'CR_JUDGE_SCRAP', true, '客退直接报废')
on conflict (source_status, target_status, scene_code) do nothing;

-- ---------------------------------------------------------------------------
-- 4. 权限点
-- ---------------------------------------------------------------------------
insert into sys.sys_permission (perm_code, perm_name, module_code, action_code) values
    ('mdm.material.view',     '物料查看',     'mdm', 'view'),
    ('mdm.material.edit',     '物料维护',     'mdm', 'edit'),
    ('mdm.warehouse.view',    '仓库查看',     'mdm', 'view'),
    ('mdm.warehouse.edit',    '仓库维护',     'mdm', 'edit'),
    ('mdm.bom.view',          'BOM查看',      'mdm', 'view'),
    ('mdm.bom.edit',          'BOM维护',      'mdm', 'edit'),
    ('mdm.route.view',        '工艺路线查看', 'mdm', 'view'),
    ('mdm.route.edit',        '工艺路线维护', 'mdm', 'edit'),
    ('inv.balance.view',      '库存查看',     'inv', 'view'),
    ('inv.txn.view',          '流水查看',     'inv', 'view'),
    ('inbound.view',          '入库查看',     'inbound', 'view'),
    ('inbound.create',        '入库录入',     'inbound', 'create'),
    ('inbound.submit',        '入库提交',     'inbound', 'submit'),
    ('inbound.void',          '入库作废',     'inbound', 'void'),
    ('outbound.view',         '出库查看',     'outbound', 'view'),
    ('outbound.create',       '出库录入',     'outbound', 'create'),
    ('outbound.submit',       '出库提交',     'outbound', 'submit'),
    ('outbound.void',         '出库作废',     'outbound', 'void'),
    ('preissue.view',         '异常先发查看', 'preissue', 'view'),
    ('preissue.create',       '异常先发录入', 'preissue', 'create'),
    ('preissue.close',        '异常闭环',     'preissue', 'close'),
    ('defect.view',           '不良查看',     'quality', 'view'),
    ('defect.create',         '不良录入',     'quality', 'create'),
    ('recovery.create',       '拆解回收',     'quality', 'create'),
    ('scrap.create',          '报废录入',     'quality', 'create'),
    ('customer_return.view',  '客退查看',     'return', 'view'),
    ('customer_return.judge', '客退判定',     'return', 'judge'),
    ('supplier_return.view',  '退供查看',     'return', 'view'),
    ('supplier_return.create','退供录入',     'return', 'create'),
    ('outsource.view',        '委外查看',     'outsource', 'view'),
    ('outsource.create',      '委外录入',     'outsource', 'create'),
    ('stocktake.view',        '盘点查看',     'stocktake', 'view'),
    ('stocktake.create',      '盘点创建',     'stocktake', 'create'),
    ('stocktake.adjust',      '盘点调整',     'stocktake', 'adjust'),
    ('sys.user.manage',       '用户管理',     'sys', 'manage'),
    ('sys.role.manage',       '角色管理',     'sys', 'manage')
on conflict (perm_code) do nothing;

-- ---------------------------------------------------------------------------
-- 5. 角色
-- ---------------------------------------------------------------------------
insert into sys.sys_role (role_code, role_name) values
    ('ADMIN',          '系统管理员'),
    ('WH_MANAGER',     '仓库主管'),
    ('WH_OPERATOR',    '仓管员'),
    ('PROD_PICKER',    '生产领料员'),
    ('QUALITY',        '质量/异常处理员'),
    ('OUTSOURCE_MGR',  '委外管理员')
on conflict (role_code) do nothing;

-- ---------------------------------------------------------------------------
-- 6. ADMIN 角色拥有所有权限
-- ---------------------------------------------------------------------------
insert into sys.sys_role_permission (role_id, permission_id)
select r.id, p.id
  from sys.sys_role r
  cross join sys.sys_permission p
 where r.role_code = 'ADMIN'
on conflict (role_id, permission_id) do nothing;

-- ---------------------------------------------------------------------------
-- 7. 初始管理员用户 (admin / Admin@123)
-- 注意: 使用 $2a$ 格式的 bcrypt 哈希，与 Rust bcrypt 库兼容
-- ---------------------------------------------------------------------------
insert into sys.sys_user (user_code, user_name, login_name, password_hash, is_active) values
    ('admin', '系统管理员', 'admin',
     '$2a$10$/GD2bPJipaCyYgvv.U9rNOgpkbaUtSEn1mvlYsZz4aW1HjmZD1nCS',
     true)
on conflict (user_code) do nothing;

insert into sys.sys_user_role (user_id, role_id)
select u.id, r.id
  from sys.sys_user u, sys.sys_role r
 where u.login_name = 'admin'
   and r.role_code  = 'ADMIN'
on conflict (user_id, role_id) do nothing;

-- ---------------------------------------------------------------------------
-- 8. 默认仓库与仓位
-- ---------------------------------------------------------------------------
insert into mdm.mdm_warehouse (wh_code, wh_name, wh_type, remark) values
    ('RAW01',     '原材料仓',       'RAW_WH',     '存放 FPC/功能片/盖板/屏幕/OCA/保护膜等原料'),
    ('SEMI01',    '半成品仓',       'SEMI_WH',    '存放 FOG / TP'),
    ('FG01',      '成品仓',         'FG_WH',      '存放总成'),
    ('BAD01',     '不良品仓',       'BAD_WH',     '不良品集中管理'),
    ('SCRAP01',   '报废仓',         'SCRAP_WH',   '已判定报废'),
    ('TRANSIT01', '委外在途仓',     'TRANSIT_WH', '发往委外的在途物料(逻辑仓)'),
    ('RETURN01',  '客退待判定仓',   'RETURN_WH',  '客户退货待判定'),
    ('CHECK01',   '待检仓',         'CHECK_WH',   '来料待 IQC 检验')
on conflict (wh_code) do nothing;

insert into mdm.mdm_location (wh_id, loc_code, loc_name, loc_type)
select w.id, 'DEFAULT', '默认仓位',
    case w.wh_type
        when 'BAD_WH'     then 'BAD'
        when 'SCRAP_WH'   then 'SCRAP'
        when 'TRANSIT_WH' then 'TRANSIT'
        when 'RETURN_WH'  then 'RETURN'
        when 'CHECK_WH'   then 'IQC'
        else 'NORMAL'
    end
from mdm.mdm_warehouse w
on conflict (wh_id, loc_code) do nothing;
