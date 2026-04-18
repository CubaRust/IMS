-- ============================================================================
-- 0011_demo_data.sql
-- 演示数据(生产环境不执行)
-- 使用:单独执行,不放进 sqlx migrate 默认流程
--   psql "$DATABASE_URL" -f migrations/0011_demo_data.sql
-- ============================================================================

-- ---------------------------------------------------------------------------
-- 1. 供应商
-- ---------------------------------------------------------------------------
insert into mdm.mdm_supplier (supplier_code, supplier_name, contact_name, contact_phone) values
    ('S001', '深圳市蓝思科技有限公司',       '张经理', '13800000001'),
    ('S002', '欧菲光科技股份有限公司',       '李经理', '13800000002'),
    ('S003', '伯恩光学(惠州)有限公司',      '王经理', '13800000003'),
    ('S004', '合力泰科技股份有限公司',       '赵经理', '13800000004'),
    ('S005', '南玻 A(深圳南玻)',             '陈经理', '13800000005'),
    ('OS001','深圳某外协加工厂(绑定 FPC)',   '林厂长', '13900000001'),
    ('OS002','东莞某外协加工厂(TP 贴合)',    '黄厂长', '13900000002')
on conflict (supplier_code) do nothing;

-- ---------------------------------------------------------------------------
-- 2. 客户
-- ---------------------------------------------------------------------------
insert into mdm.mdm_customer (customer_code, customer_name, contact_name) values
    ('C001', '某品牌手机代工厂 A', '采购张'),
    ('C002', '某品牌手机代工厂 B', '采购李'),
    ('C003', '某售后市场客户 A',   '采购王')
on conflict (customer_code) do nothing;

-- ---------------------------------------------------------------------------
-- 3. 物料
-- ---------------------------------------------------------------------------

-- 3.1 原材料
insert into mdm.mdm_material
    (material_code, material_name, material_category, spec_model, brand, unit,
     process_type, has_ic_flag, key_material_flag, batch_required_flag, safety_stock, min_stock)
values
    ('RM-FPC-IC-001',    'FPC(带触摸IC)-6.1寸', 'RAW', '6.1英寸带IC', '厂商A', 'PCS', null, true,  true,  true, 500, 100),
    ('RM-FPC-NOIC-001',  'FPC(无IC)-6.1寸',     'RAW', '6.1英寸无IC', '厂商A', 'PCS', null, false, true,  true, 500, 100),
    ('RM-FP-001',        '功能片-6.1寸',        'RAW', '6.1英寸',     '厂商B', 'PCS', null, false, true,  true, 800, 200),
    ('RM-GLASS-001',     '玻璃盖板-6.1寸-品牌A','RAW', '6.1英寸',     '蓝思', 'PCS', null, false, true, true, 1000, 200),
    ('RM-GLASS-002',     '玻璃盖板-6.1寸-品牌B','RAW', '6.1英寸',     '伯恩', 'PCS', null, false, true, true, 1000, 200),
    ('RM-LCD-001',       '屏幕-6.1寸',          'RAW', '6.1英寸',     '厂商C', 'PCS', null, false, true, true, 800,  150),
    ('RM-OCA-001',       'OCA胶',               'PUBLIC', '标准规格', '厂商D', 'PCS', null, false, false, true, 2000, 500),
    ('RM-FILM-001',      '保护膜',              'PUBLIC', '标准规格', '厂商E', 'PCS', null, false, false, true, 3000, 500)
on conflict (tenant_id, material_code) do nothing;

-- 3.2 半成品
insert into mdm.mdm_material
    (material_code, material_name, material_category, spec_model, unit,
     process_type, key_material_flag, batch_required_flag, allow_recovery_flag)
values
    ('SEMI-FOG-001', 'FOG-6.1寸', 'SEMI', '6.1英寸', 'PCS', 'FOG', true, true, true),
    ('SEMI-TP-001',  'TP-6.1寸',  'SEMI', '6.1英寸', 'PCS', 'TP',  true, true, true)
on conflict (tenant_id, material_code) do nothing;

-- 3.3 成品
insert into mdm.mdm_material
    (material_code, material_name, material_category, spec_model, unit,
     process_type, key_material_flag, batch_required_flag, allow_recovery_flag)
values
    ('FG-ASM-001', '总成-6.1寸-路线1(TP+屏幕)',  'FINISHED', '6.1英寸', 'PCS', 'ASM', true, true, true),
    ('FG-ASM-002', '总成-6.1寸-路线2(盖板+屏幕)','FINISHED', '6.1英寸', 'PCS', 'ASM', true, true, true)
on conflict (tenant_id, material_code) do nothing;

-- ---------------------------------------------------------------------------
-- 4. 工艺路线
-- ---------------------------------------------------------------------------
insert into mdm.mdm_route_h (route_code, route_name, product_material_id)
select 'RT-ASM-001', '总成路线1:分步贴合',
       (select id from mdm.mdm_material where material_code = 'FG-ASM-001')
on conflict (route_code) do nothing;

insert into mdm.mdm_route_d (route_id, step_no, process_name, output_material_id, semi_finished_flag)
select
    (select id from mdm.mdm_route_h where route_code = 'RT-ASM-001'),
    t.step_no, t.process_name,
    (select id from mdm.mdm_material where material_code = t.output_code),
    t.semi
from (values
    (1, '绑定',       'SEMI-FOG-001', true),
    (2, 'TP贴合',     'SEMI-TP-001',  true),
    (3, '总成贴合',   'FG-ASM-001',   false)
) as t(step_no, process_name, output_code, semi)
on conflict (route_id, step_no) do nothing;

insert into mdm.mdm_route_h (route_code, route_name, product_material_id)
select 'RT-ASM-002', '总成路线2:盖板+屏幕直接总成',
       (select id from mdm.mdm_material where material_code = 'FG-ASM-002')
on conflict (route_code) do nothing;

insert into mdm.mdm_route_d (route_id, step_no, process_name, output_material_id, semi_finished_flag)
select
    (select id from mdm.mdm_route_h where route_code = 'RT-ASM-002'),
    1, '总成贴合',
    (select id from mdm.mdm_material where material_code = 'FG-ASM-002'),
    false
on conflict (route_id, step_no) do nothing;

-- ---------------------------------------------------------------------------
-- 5. BOM
-- ---------------------------------------------------------------------------
insert into mdm.mdm_bom_h (bom_code, bom_version, product_material_id, route_id)
select 'BOM-ASM-001', 'V1',
       (select id from mdm.mdm_material where material_code = 'FG-ASM-001'),
       (select id from mdm.mdm_route_h where route_code = 'RT-ASM-001')
on conflict (bom_code) do nothing;

insert into mdm.mdm_bom_d (bom_id, line_no, material_id, usage_qty, public_material_flag)
select (select id from mdm.mdm_bom_h where bom_code = 'BOM-ASM-001'),
       t.line_no,
       (select id from mdm.mdm_material where material_code = t.mat_code),
       t.usage_qty,
       t.is_public
from (values
    (1, 'SEMI-TP-001',   1, false),
    (2, 'RM-LCD-001',    1, false),
    (3, 'RM-OCA-001',    1, true),
    (4, 'RM-FILM-001',   1, true)
) as t(line_no, mat_code, usage_qty, is_public)
on conflict (bom_id, line_no) do nothing;
