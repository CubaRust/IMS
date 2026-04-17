-- ============================================================================
-- 0004_inbound_outbound.sql
-- 入库单 + 出库单
-- ============================================================================

-- ---------------------------------------------------------------------------
-- 1. 入库单头
-- ---------------------------------------------------------------------------
create table if not exists wms.wms_inbound_h (
    id                  bigserial primary key,
    inbound_no          varchar(50) not null unique,
    inbound_type        varchar(30) not null,
    supplier_id         bigint references mdm.mdm_supplier(id),
    source_object_type  varchar(50),
    source_object_id    bigint,
    wh_id               bigint not null references mdm.mdm_warehouse(id),
    loc_id              bigint references mdm.mdm_location(id),
    inbound_date        date not null,
    operator_id         bigint references sys.sys_user(id),
    doc_status          varchar(20) not null default 'DRAFT',
    extra_json          jsonb not null default '{}'::jsonb,
    remark              text,
    created_at          timestamp not null default now(),
    updated_at          timestamp not null default now(),
    check (inbound_type in ('PURCHASE','PROD','RETURN','OUTSOURCE_BACK','CUSTOMER_RETURN','RECOVERY_IN','OTHER')),
    check (doc_status in ('DRAFT','SUBMITTED','COMPLETED','VOIDED'))
);
create index if not exists idx_inbound_h_date on wms.wms_inbound_h(inbound_date);
create index if not exists idx_inbound_h_supplier on wms.wms_inbound_h(supplier_id);
create index if not exists idx_inbound_h_type on wms.wms_inbound_h(inbound_type);
create index if not exists idx_inbound_h_status on wms.wms_inbound_h(doc_status);
create index if not exists idx_inbound_h_wh on wms.wms_inbound_h(wh_id);
comment on table wms.wms_inbound_h is '入库单头';
comment on column wms.wms_inbound_h.inbound_type is 'PURCHASE/PROD/RETURN/OUTSOURCE_BACK/CUSTOMER_RETURN/RECOVERY_IN/OTHER';

create trigger trg_wms_inbound_h_updated_at
    before update on wms.wms_inbound_h
    for each row execute function sys.fn_set_updated_at();

-- ---------------------------------------------------------------------------
-- 2. 入库单行
-- ---------------------------------------------------------------------------
create table if not exists wms.wms_inbound_d (
    id                          bigserial primary key,
    inbound_id                  bigint not null references wms.wms_inbound_h(id) on delete cascade,
    line_no                     integer not null,
    material_id                 bigint not null references mdm.mdm_material(id),
    batch_no                    varchar(100) not null default '',
    qty                         numeric(18,4) not null,
    unit                        varchar(30) not null,
    stock_status                varchar(30) not null,
    work_order_no               varchar(50),
    process_name                varchar(100),
    outsource_no                varchar(50),
    related_preissue_line_id    bigint,
    note                        text,
    created_at                  timestamp not null default now(),
    unique(inbound_id, line_no),
    check (qty > 0)
);
create index if not exists idx_inbound_d_material on wms.wms_inbound_d(material_id);
create index if not exists idx_inbound_d_inbound on wms.wms_inbound_d(inbound_id);
create index if not exists idx_inbound_d_preissue on wms.wms_inbound_d(related_preissue_line_id) where related_preissue_line_id is not null;
create index if not exists idx_inbound_d_work_order on wms.wms_inbound_d(work_order_no) where work_order_no is not null;
comment on table wms.wms_inbound_d is '入库单行';
comment on column wms.wms_inbound_d.related_preissue_line_id is '关联的待入库先发行(用于异常闭环冲销)';

-- ---------------------------------------------------------------------------
-- 3. 出库单头
-- ---------------------------------------------------------------------------
create table if not exists wms.wms_outbound_h (
    id                  bigserial primary key,
    outbound_no         varchar(50) not null unique,
    outbound_type       varchar(30) not null,
    target_object_type  varchar(50),
    target_object_id    bigint,
    work_order_no       varchar(50),
    process_name        varchar(100),
    route_id            bigint references mdm.mdm_route_h(id),
    workshop_name       varchar(100),
    wh_id               bigint not null references mdm.mdm_warehouse(id),
    outbound_date       date not null,
    operator_id         bigint references sys.sys_user(id),
    doc_status          varchar(20) not null default 'DRAFT',
    extra_json          jsonb not null default '{}'::jsonb,
    remark              text,
    created_at          timestamp not null default now(),
    updated_at          timestamp not null default now(),
    check (outbound_type in ('PROD_ISSUE','PROCESS_ISSUE','PUBLIC_ISSUE','OUTSOURCE_SEND','SUPPLIER_RETURN','SALES_SEND','SCRAP_OUT','OTHER')),
    check (doc_status in ('DRAFT','SUBMITTED','COMPLETED','VOIDED'))
);
create index if not exists idx_outbound_h_date on wms.wms_outbound_h(outbound_date);
create index if not exists idx_outbound_h_type on wms.wms_outbound_h(outbound_type);
create index if not exists idx_outbound_h_status on wms.wms_outbound_h(doc_status);
create index if not exists idx_outbound_h_work_order on wms.wms_outbound_h(work_order_no) where work_order_no is not null;
create index if not exists idx_outbound_h_wh on wms.wms_outbound_h(wh_id);
comment on table wms.wms_outbound_h is '出库单头';

create trigger trg_wms_outbound_h_updated_at
    before update on wms.wms_outbound_h
    for each row execute function sys.fn_set_updated_at();

-- ---------------------------------------------------------------------------
-- 4. 出库单行
-- ---------------------------------------------------------------------------
create table if not exists wms.wms_outbound_d (
    id                      bigserial primary key,
    outbound_id             bigint not null references wms.wms_outbound_h(id) on delete cascade,
    line_no                 integer not null,
    material_id             bigint not null references mdm.mdm_material(id),
    batch_no                varchar(100) not null default '',
    suggest_qty             numeric(18,4) not null default 0,
    actual_qty              numeric(18,4) not null,
    unit                    varchar(30) not null,
    stock_status            varchar(30) not null default 'QUALIFIED',
    bom_recommended_flag    boolean not null default false,
    public_material_flag    boolean not null default false,
    preissue_flag           boolean not null default false,
    note                    text,
    created_at              timestamp not null default now(),
    unique(outbound_id, line_no),
    check (actual_qty > 0)
);
create index if not exists idx_outbound_d_outbound on wms.wms_outbound_d(outbound_id);
create index if not exists idx_outbound_d_material on wms.wms_outbound_d(material_id);
comment on table wms.wms_outbound_d is '出库单行';
comment on column wms.wms_outbound_d.preissue_flag is '是否异常先发(关联 wms_preissue)';
