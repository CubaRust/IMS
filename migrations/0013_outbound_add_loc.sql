-- ============================================================================
-- 0013_outbound_add_loc.sql
-- 给 wms_outbound_h 补 loc_id,对齐 wms_inbound_h 的设计
--
-- 背景:原 0004 设计时 outbound_h 只记 wh_id(认为每行 loc 可以不同)。
-- 实际业务里 99% 的出库场景单据是从同一个仓位发,放到 head 级更清晰。
-- 后续若需要行级 loc,在 outbound_d 再补字段(暂不做)。
-- ============================================================================

alter table wms.wms_outbound_h
    add column if not exists loc_id bigint references mdm.mdm_location(id);

create index if not exists idx_outbound_h_loc on wms.wms_outbound_h(loc_id) where loc_id is not null;

comment on column wms.wms_outbound_h.loc_id is '单据级源仓位(0013 补)';
