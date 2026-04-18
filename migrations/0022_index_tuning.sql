-- ============================================================================
-- 0022_index_tuning.sql
-- 索引优化:v_today_io 表达式索引 + defect 复合索引
-- ============================================================================

-- 1. v_today_io 依赖 operate_time::date = current_date
--    btree(operate_time) 无法被 ::date 类型转换利用,需要表达式索引
create index if not exists idx_inv_txn_h_operate_date
    on wms.wms_inventory_txn_h ((operate_time::date));

-- 2. v_defect_stats_30d: found_date >= current_date - 30 AND doc_status <> 'VOIDED'
--    found_date 已有单列索引,加一个复合索引覆盖两个过滤条件
create index if not exists idx_defect_h_date_status
    on wms.wms_defect_h (found_date, doc_status);

-- 3. dashboard: count(distinct material_id) from wms_inventory_balance where book_qty > 0
--    加一个部分索引,只索引有库存的行
create index if not exists idx_inv_balance_positive_material
    on wms.wms_inventory_balance (material_id)
    where book_qty > 0;
