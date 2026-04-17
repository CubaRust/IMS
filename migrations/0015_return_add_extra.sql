-- ============================================================================
-- 0015_return_add_extra.sql
-- 给客户退货 / 供应商退货 / 委外单 三张 head 加 extra_json
-- 用以存储源/目标仓位等扩展字段(这几张表原 DDL 里没有 extra_json)
-- ============================================================================

alter table wms.wms_customer_return_h
    add column if not exists extra_json jsonb not null default '{}'::jsonb;
comment on column wms.wms_customer_return_h.extra_json is '扩展:return_wh/loc、defect_wh/loc、scrap_wh/loc 等';

alter table wms.wms_supplier_return_h
    add column if not exists extra_json jsonb not null default '{}'::jsonb;
comment on column wms.wms_supplier_return_h.extra_json is '扩展:source_wh/loc 等';

alter table wms.wms_outsource_h
    add column if not exists extra_json jsonb not null default '{}'::jsonb;
comment on column wms.wms_outsource_h.extra_json is '扩展:send_wh/loc、back_wh/loc、scrap_wh/loc 等';
