-- ============================================================================
-- 0009_reporting.sql
-- 报表视图(rpt.*)
--
-- 原则:
--   - 视图只读,不用于业务写入
--   - 复杂报表优先走视图,前端直接 select
-- ============================================================================

-- ---------------------------------------------------------------------------
-- 1. 当前库存视图(物料级汇总)
-- ---------------------------------------------------------------------------
create or replace view rpt.v_inventory_by_material as
select
    m.id                as material_id,
    m.material_code,
    m.material_name,
    m.material_category,
    m.process_type,
    m.brand,
    m.unit,
    m.safety_stock,
    m.min_stock,
    coalesce(sum(b.book_qty),      0) as book_qty_total,
    coalesce(sum(b.available_qty), 0) as available_qty_total,
    coalesce(sum(b.occupied_qty),  0) as occupied_qty_total,
    coalesce(sum(b.bad_qty),       0) as bad_qty_total,
    coalesce(sum(b.scrap_qty),     0) as scrap_qty_total,
    coalesce(sum(b.pending_qty),   0) as pending_qty_total
from mdm.mdm_material m
left join wms.wms_inventory_balance b on b.material_id = m.id
where m.is_active = true
group by m.id;

comment on view rpt.v_inventory_by_material is '按物料汇总的库存视图';

-- ---------------------------------------------------------------------------
-- 2. 当前库存视图(仓位级明细)
-- ---------------------------------------------------------------------------
create or replace view rpt.v_inventory_by_location as
select
    b.id,
    m.material_code,
    m.material_name,
    m.process_type,
    m.brand,
    w.wh_code,
    w.wh_name,
    l.loc_code,
    l.loc_name,
    b.batch_no,
    b.stock_status,
    b.book_qty,
    b.available_qty,
    b.occupied_qty,
    b.bad_qty,
    b.scrap_qty,
    b.pending_qty,
    b.updated_at
from wms.wms_inventory_balance b
join mdm.mdm_material  m on m.id = b.material_id
join mdm.mdm_warehouse w on w.id = b.wh_id
join mdm.mdm_location  l on l.id = b.loc_id;

comment on view rpt.v_inventory_by_location is '按仓位的库存明细视图';

-- ---------------------------------------------------------------------------
-- 3. 低库存预警
-- ---------------------------------------------------------------------------
create or replace view rpt.v_low_stock_warning as
select
    mat.material_id,
    mat.material_code,
    mat.material_name,
    mat.material_category,
    mat.unit,
    mat.safety_stock,
    mat.min_stock,
    mat.available_qty_total,
    case
        when mat.available_qty_total <= mat.min_stock    then 'CRITICAL'
        when mat.available_qty_total <= mat.safety_stock then 'WARNING'
        else 'NORMAL'
    end as warning_level
from rpt.v_inventory_by_material mat
where mat.safety_stock > 0
  and mat.available_qty_total <= mat.safety_stock;

comment on view rpt.v_low_stock_warning is '低库存预警(低于安全库存的物料)';

-- ---------------------------------------------------------------------------
-- 4. 异常待办视图
-- ---------------------------------------------------------------------------
create or replace view rpt.v_anomaly_todo as
-- 待补入库(异常先发)
select
    'PREISSUE'::varchar       as anomaly_type,
    h.id                      as doc_id,
    h.preissue_no             as doc_no,
    h.issue_date              as event_date,
    h.supplier_id,
    h.work_order_no,
    h.reason                  as reason,
    h.exception_status        as status,
    h.timeout_flag,
    h.created_at
from wms.wms_preissue_h h
where h.exception_status in ('PENDING','PARTIAL','OVERTIME')

union all
-- 待处理不良
select
    'DEFECT_PENDING'::varchar,
    h.id,
    h.defect_no,
    h.found_date,
    null::bigint,
    h.work_order_no,
    h.process_method,
    h.doc_status,
    false,
    h.created_at
from wms.wms_defect_h h
where h.doc_status in ('DRAFT','SUBMITTED')

union all
-- 待判定客退
select
    'CUSTOMER_RETURN_PENDING_JUDGE'::varchar,
    h.id,
    h.customer_return_no,
    h.return_date,
    null::bigint,
    null::varchar,
    h.return_reason,
    h.doc_status,
    false,
    h.created_at
from wms.wms_customer_return_h h
where h.doc_status in ('DRAFT','SUBMITTED')

union all
-- 盘点差异待确认
select
    'STOCKTAKE_DIFF'::varchar,
    h.id,
    h.stocktake_no,
    h.stocktake_date,
    null::bigint,
    null::varchar,
    null::text,
    h.doc_status,
    false,
    h.created_at
from wms.wms_stocktake_h h
where h.doc_status = 'COUNTING';

comment on view rpt.v_anomaly_todo is '异常待办聚合视图(异常中心用)';

-- ---------------------------------------------------------------------------
-- 5. 当日出入库视图
-- ---------------------------------------------------------------------------
create or replace view rpt.v_today_io as
select
    h.id as txn_id,
    h.txn_no,
    h.txn_type,
    h.scene_code,
    h.doc_type,
    h.doc_no,
    d.material_id,
    m.material_code,
    m.material_name,
    d.batch_no,
    d.qty,
    d.unit,
    d.io_flag,
    h.source_wh_id,
    h.target_wh_id,
    h.operator_id,
    h.operate_time
from wms.wms_inventory_txn_h h
join wms.wms_inventory_txn_d d on d.txn_id = h.id
join mdm.mdm_material m on m.id = d.material_id
where h.operate_time::date = current_date;

comment on view rpt.v_today_io is '当日出入库流水';

-- ---------------------------------------------------------------------------
-- 6. 不良统计视图(近30天)
-- ---------------------------------------------------------------------------
create or replace view rpt.v_defect_stats_30d as
select
    m.material_code,
    m.material_name,
    h.defect_source,
    h.product_stage,
    h.process_method,
    count(d.id)                 as line_count,
    coalesce(sum(d.qty), 0)     as total_qty,
    max(h.found_date)           as last_found_date
from wms.wms_defect_h h
join wms.wms_defect_d d on d.defect_id = h.id
join mdm.mdm_material m on m.id = d.material_id
where h.found_date >= current_date - interval '30 days'
  and h.doc_status <> 'VOIDED'
group by m.material_code, m.material_name, h.defect_source, h.product_stage, h.process_method;

comment on view rpt.v_defect_stats_30d is '不良统计(近30天)';

-- ---------------------------------------------------------------------------
-- 7. 委外在途视图
-- ---------------------------------------------------------------------------
create or replace view rpt.v_outsource_in_transit as
select
    h.id                            as outsource_id,
    h.outsource_no,
    s.supplier_code,
    s.supplier_name,
    h.work_order_no,
    h.process_name,
    h.send_date,
    h.expect_back_date,
    h.doc_status,
    coalesce(sum(send_d.sent_qty), 0)       as total_sent_qty,
    coalesce(sum(back_d.received_qty), 0)   as total_received_qty,
    coalesce(sum(send_d.sent_qty), 0) - coalesce(sum(back_d.received_qty), 0) as in_transit_qty
from wms.wms_outsource_h h
join mdm.mdm_supplier s on s.id = h.supplier_id
left join wms.wms_outsource_send_d send_d on send_d.outsource_id = h.id
left join wms.wms_outsource_back_d back_d on back_d.outsource_id = h.id
where h.doc_status in ('SENT','PARTIAL_BACK')
group by h.id, s.supplier_code, s.supplier_name;

comment on view rpt.v_outsource_in_transit is '委外在途统计';
