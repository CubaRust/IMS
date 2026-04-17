-- ============================================================================
-- verify.sql
-- 数据库 schema 完整性自检脚本
-- 用法:psql "$DATABASE_URL" -f scripts/verify.sql
--
-- 输出:
--   - schema / 表 / 视图 / 函数 / 触发器 统计
--   - 关键业务表记录数
--   - 最近错误:缺失的表、缺失的索引
-- ============================================================================

\echo '========== Schema 统计 =========='
select nspname as schema, count(*) filter (where relkind = 'r') as tables,
       count(*) filter (where relkind = 'v') as views
  from pg_class c
  join pg_namespace n on n.oid = c.relnamespace
 where nspname in ('sys','mdm','wms','rpt')
 group by nspname
 order by nspname;

\echo ''
\echo '========== 关键表存在性 =========='
select relname as missing_table
  from (values
    ('sys','sys_user'), ('sys','sys_role'), ('sys','sys_permission'),
    ('sys','sys_user_role'), ('sys','sys_role_permission'),
    ('sys','sys_doc_no_rule'), ('sys','sys_dict'),
    ('mdm','mdm_material'), ('mdm','mdm_warehouse'), ('mdm','mdm_location'),
    ('mdm','mdm_supplier'), ('mdm','mdm_customer'),
    ('mdm','mdm_bom_h'), ('mdm','mdm_bom_d'),
    ('mdm','mdm_route_h'), ('mdm','mdm_route_d'),
    ('mdm','mdm_status_flow'),
    ('mdm','mdm_recovery_tpl_h'), ('mdm','mdm_recovery_tpl_d'),
    ('wms','wms_inventory_balance'),
    ('wms','wms_inventory_txn_h'), ('wms','wms_inventory_txn_d'),
    ('wms','wms_inbound_h'), ('wms','wms_inbound_d'),
    ('wms','wms_outbound_h'), ('wms','wms_outbound_d'),
    ('wms','wms_preissue_h'), ('wms','wms_preissue_d'),
    ('wms','wms_defect_h'), ('wms','wms_defect_d'),
    ('wms','wms_recovery_h'), ('wms','wms_recovery_in'),
    ('wms','wms_recovery_out'), ('wms','wms_recovery_scrap'),
    ('wms','wms_scrap_h'), ('wms','wms_scrap_d'),
    ('wms','wms_customer_return_h'), ('wms','wms_customer_return_d'),
    ('wms','wms_customer_return_judge'),
    ('wms','wms_supplier_return_h'), ('wms','wms_supplier_return_d'),
    ('wms','wms_outsource_h'), ('wms','wms_outsource_send_d'), ('wms','wms_outsource_back_d'),
    ('wms','wms_stocktake_h'), ('wms','wms_stocktake_d')
  ) as expected(sch, tbl)
  left join pg_class c on c.relname = expected.tbl
  left join pg_namespace n on n.oid = c.relnamespace and n.nspname = expected.sch
 where c.oid is null;

\echo ''
\echo '========== 关键视图存在性 =========='
select v.viewname as missing_view
  from (values
    ('v_inventory_by_material'), ('v_inventory_by_location'),
    ('v_low_stock_warning'), ('v_anomaly_todo'),
    ('v_today_io'), ('v_defect_stats_30d'), ('v_outsource_in_transit')
  ) as expected(viewname)
  left join pg_views v on v.viewname = expected.viewname and v.schemaname = 'rpt'
 where v.viewname is null;

\echo ''
\echo '========== 关键函数存在性 =========='
select case when count(*) = 0 then 'MISSING: sys.fn_next_doc_no' else 'OK' end as fn_next_doc_no
  from pg_proc p join pg_namespace n on n.oid = p.pronamespace
 where n.nspname = 'sys' and p.proname = 'fn_next_doc_no';

\echo ''
\echo '========== 种子数据统计 =========='
select 'sys.sys_user'          as tbl, count(*) from sys.sys_user
union all
select 'sys.sys_role',               count(*) from sys.sys_role
union all
select 'sys.sys_permission',         count(*) from sys.sys_permission
union all
select 'sys.sys_role_permission',    count(*) from sys.sys_role_permission
union all
select 'sys.sys_user_role',          count(*) from sys.sys_user_role
union all
select 'sys.sys_doc_no_rule',        count(*) from sys.sys_doc_no_rule
union all
select 'sys.sys_dict',               count(*) from sys.sys_dict
union all
select 'mdm.mdm_warehouse',          count(*) from mdm.mdm_warehouse
union all
select 'mdm.mdm_location',           count(*) from mdm.mdm_location
union all
select 'mdm.mdm_status_flow',        count(*) from mdm.mdm_status_flow
union all
select 'mdm.mdm_material',           count(*) from mdm.mdm_material
union all
select 'mdm.mdm_supplier',           count(*) from mdm.mdm_supplier
union all
select 'mdm.mdm_customer',           count(*) from mdm.mdm_customer
union all
select 'mdm.mdm_bom_h',              count(*) from mdm.mdm_bom_h
union all
select 'mdm.mdm_route_h',            count(*) from mdm.mdm_route_h
order by tbl;

\echo ''
\echo '========== 编码规则自测(生成一个入库单号看) =========='
select sys.fn_next_doc_no('INBOUND') as sample_inbound_no;
-- 还原序号(避免影响真实业务编号)
update sys.sys_doc_no_rule
   set current_seq = greatest(current_seq - 1, 0)
 where doc_type = 'INBOUND';

\echo ''
\echo '========== 验证完成 =========='
