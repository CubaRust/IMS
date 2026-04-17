-- ============================================================================
-- 0006_defect_recovery.sql
-- 生产不良 / 来料不良 + 拆解回收
--
-- 关键规则(需求文档 16 节):
--   - FOG/TP/总成 NG 不能走普通退料,必须走不良单
--   - 不良 4 种处理方式:转不良库/拆解/报废/返工
-- ============================================================================

-- ---------------------------------------------------------------------------
-- 1. 不良单头
-- ---------------------------------------------------------------------------
create table if not exists wms.wms_defect_h (
    id                  bigserial primary key,
    defect_no           varchar(50) not null unique,
    defect_source       varchar(30) not null,
    work_order_no       varchar(50),
    process_name        varchar(100),
    product_stage       varchar(20) not null,
    found_date          date not null,
    finder_name         varchar(100),
    process_method      varchar(30) not null,
    operator_id         bigint references sys.sys_user(id),
    doc_status          varchar(20) not null default 'DRAFT',
    extra_json          jsonb not null default '{}'::jsonb,
    remark              text,
    created_at          timestamp not null default now(),
    updated_at          timestamp not null default now(),
    check (defect_source in ('IQC','PROD','CUSTOMER_RETURN','OUTSOURCE')),
    check (product_stage in ('RAW','FOG','TP','ASM')),
    check (process_method in ('TO_BAD_STOCK','TO_DISMANTLE','TO_SCRAP','TO_REWORK')),
    check (doc_status in ('DRAFT','SUBMITTED','COMPLETED','VOIDED'))
);
create index if not exists idx_defect_h_date on wms.wms_defect_h(found_date);
create index if not exists idx_defect_h_source on wms.wms_defect_h(defect_source);
create index if not exists idx_defect_h_stage on wms.wms_defect_h(product_stage);
create index if not exists idx_defect_h_status on wms.wms_defect_h(doc_status);
create index if not exists idx_defect_h_work_order on wms.wms_defect_h(work_order_no) where work_order_no is not null;
comment on table wms.wms_defect_h is '不良单头';
comment on column wms.wms_defect_h.defect_source is 'IQC来料/PROD生产/CUSTOMER_RETURN客退/OUTSOURCE委外';
comment on column wms.wms_defect_h.product_stage is 'RAW原料/FOG/TP/ASM总成';
comment on column wms.wms_defect_h.process_method is 'TO_BAD_STOCK转不良库/TO_DISMANTLE拆解/TO_SCRAP报废/TO_REWORK返工';

create trigger trg_wms_defect_h_updated_at
    before update on wms.wms_defect_h
    for each row execute function sys.fn_set_updated_at();

-- ---------------------------------------------------------------------------
-- 2. 不良单行
-- ---------------------------------------------------------------------------
create table if not exists wms.wms_defect_d (
    id                  bigserial primary key,
    defect_id           bigint not null references wms.wms_defect_h(id) on delete cascade,
    line_no             integer not null,
    material_id         bigint not null references mdm.mdm_material(id),
    batch_no            varchar(100) not null default '',
    qty                 numeric(18,4) not null,
    unit                varchar(30) not null,
    defect_reason       varchar(50) not null,
    defect_desc         text,
    source_doc_type     varchar(50),
    source_doc_no       varchar(50),
    target_status       varchar(30),
    note                text,
    created_at          timestamp not null default now(),
    unique(defect_id, line_no),
    check (qty > 0)
);
create index if not exists idx_defect_d_defect on wms.wms_defect_d(defect_id);
create index if not exists idx_defect_d_material on wms.wms_defect_d(material_id);
comment on table wms.wms_defect_d is '不良单行';

-- ---------------------------------------------------------------------------
-- 3. 拆解回收单头
-- ---------------------------------------------------------------------------
create table if not exists wms.wms_recovery_h (
    id                  bigserial primary key,
    recovery_no         varchar(50) not null unique,
    source_defect_id    bigint not null references wms.wms_defect_h(id),
    tpl_id              bigint references mdm.mdm_recovery_tpl_h(id),
    recovery_date       date not null,
    operator_id         bigint references sys.sys_user(id),
    doc_status          varchar(20) not null default 'DRAFT',
    extra_json          jsonb not null default '{}'::jsonb,
    remark              text,
    created_at          timestamp not null default now(),
    updated_at          timestamp not null default now(),
    check (doc_status in ('DRAFT','SUBMITTED','COMPLETED','VOIDED'))
);
create index if not exists idx_recovery_h_date on wms.wms_recovery_h(recovery_date);
create index if not exists idx_recovery_h_defect on wms.wms_recovery_h(source_defect_id);
create index if not exists idx_recovery_h_status on wms.wms_recovery_h(doc_status);
comment on table wms.wms_recovery_h is '拆解回收单头';

create trigger trg_wms_recovery_h_updated_at
    before update on wms.wms_recovery_h
    for each row execute function sys.fn_set_updated_at();

-- ---------------------------------------------------------------------------
-- 4. 拆解回收:输入行(被拆的 NG 品)
-- ---------------------------------------------------------------------------
create table if not exists wms.wms_recovery_in (
    id                  bigserial primary key,
    recovery_id         bigint not null references wms.wms_recovery_h(id) on delete cascade,
    line_no             integer not null,
    material_id         bigint not null references mdm.mdm_material(id),
    batch_no            varchar(100) not null default '',
    qty                 numeric(18,4) not null,
    unit                varchar(30) not null,
    note                text,
    unique(recovery_id, line_no),
    check (qty > 0)
);
create index if not exists idx_recovery_in_recovery on wms.wms_recovery_in(recovery_id);
create index if not exists idx_recovery_in_material on wms.wms_recovery_in(material_id);
comment on table wms.wms_recovery_in is '拆解输入行';

-- ---------------------------------------------------------------------------
-- 5. 拆解回收:输出行
-- ---------------------------------------------------------------------------
create table if not exists wms.wms_recovery_out (
    id                  bigserial primary key,
    recovery_id         bigint not null references wms.wms_recovery_h(id) on delete cascade,
    line_no             integer not null,
    material_id         bigint not null references mdm.mdm_material(id),
    qty                 numeric(18,4) not null,
    unit                varchar(30) not null,
    target_wh_id        bigint references mdm.mdm_warehouse(id),
    target_loc_id       bigint references mdm.mdm_location(id),
    target_status       varchar(30) not null,
    note                text,
    unique(recovery_id, line_no),
    check (qty >= 0)
);
create index if not exists idx_recovery_out_recovery on wms.wms_recovery_out(recovery_id);
create index if not exists idx_recovery_out_material on wms.wms_recovery_out(material_id);
comment on table wms.wms_recovery_out is '拆解输出行(可回收件)';

-- ---------------------------------------------------------------------------
-- 6. 拆解回收:报废行
-- ---------------------------------------------------------------------------
create table if not exists wms.wms_recovery_scrap (
    id                  bigserial primary key,
    recovery_id         bigint not null references wms.wms_recovery_h(id) on delete cascade,
    line_no             integer not null,
    material_id         bigint references mdm.mdm_material(id),
    qty                 numeric(18,4) not null,
    unit                varchar(30) not null,
    scrap_reason        varchar(50),
    note                text,
    unique(recovery_id, line_no),
    check (qty >= 0)
);
create index if not exists idx_recovery_scrap_recovery on wms.wms_recovery_scrap(recovery_id);
comment on table wms.wms_recovery_scrap is '拆解报废行';
