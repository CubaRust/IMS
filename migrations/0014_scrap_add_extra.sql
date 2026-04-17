-- ============================================================================
-- 0014_scrap_add_extra.sql
-- 给 wms_scrap_h 加 extra_json,用来放源仓位/目标(报废)仓位等扩展信息。
-- 原 0007 设计时没留 extra_json 字段,本补丁补齐。
-- ============================================================================

alter table wms.wms_scrap_h
    add column if not exists extra_json jsonb not null default '{}'::jsonb;

comment on column wms.wms_scrap_h.extra_json is '扩展:source_wh_id/source_loc_id/scrap_wh_id/scrap_loc_id 等';
