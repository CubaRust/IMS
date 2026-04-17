-- ============================================================================
-- 0008_outsource_stocktake.sql
-- 委外加工 + 盘点
--
-- 委外设计:
--   - 委外发料:走 wms_outbound_h (outbound_type='OUTSOURCE_SEND')
--   - 委外在途:独立状态 OUTSOURCE
--   - 委外回料:走 wms_inbound_h (inbound_type='OUTSOURCE_BACK')
--   - 本表 wms_outsource_h 做发料/回料对账
-- ============================================================================

-- ---------------------------------------------------------------------------
-- 1. 委外加工单头
-- ---------------------------------------------------------------------------
create table if not exists wms.wms_outsource_h (
    id                  bigserial primary key,
    outsource_no        varchar(50) not null unique,
    supplier_id         bigint not null references mdm.mdm_supplier(id),
    work_order_no       varchar(50),
    process_name        varchar(100),
    send_date           date,
    expect_back_date    date,
    doc_status          varchar(20) not null default 'DRAFT',
    remark              text,
    created_at          timestamp not null default now(),
    updated_at          timestamp not null default now(),
    check (doc_status in ('DRAFT','SENT','PARTIAL_BACK','CLOSED','VOIDED'))
);
create index if not exists idx_outsource_h_supplier on wms.wms_outsource_h(supplier_id);
create index if not exists idx_outsource_h_status on wms.wms_outsource_h(doc_status);
create index if not exists idx_outsource_h_send_date on wms.wms_outsource_h(send_date);
comment on table wms.wms_outsource_h is '委外加工单';

create trigger trg_wms_outsource_h_updated_at
    before update on wms.wms_outsource_h
    for each row execute function sys.fn_set_updated_at();

-- ---------------------------------------------------------------------------
-- 2. 委外发料行
-- ---------------------------------------------------------------------------
create table if not exists wms.wms_outsource_send_d (
    id                      bigserial primary key,
    outsource_id            bigint not null references wms.wms_outsource_h(id) on delete cascade,
    line_no                 integer not null,
    material_id             bigint not null references mdm.mdm_material(id),
    batch_no                varchar(100) not null default '',
    expected_qty            numeric(18,4) not null,
    sent_qty                numeric(18,4) not null default 0,
    unit                    varchar(30) not null,
    outbound_line_id        bigint references wms.wms_outbound_d(id),
    note                    text,
    created_at              timestamp not null default now(),
    unique(outsource_id, line_no),
    check (expected_qty > 0),
    check (sent_qty >= 0)
);
create index if not exists idx_outsource_send_outsource on wms.wms_outsource_send_d(outsource_id);
create index if not exists idx_outsource_send_material on wms.wms_outsource_send_d(material_id);
create index if not exists idx_outsource_send_outbound on wms.wms_outsource_send_d(outbound_line_id) where outbound_line_id is not null;
comment on table wms.wms_outsource_send_d is '委外发料行';

-- ---------------------------------------------------------------------------
-- 3. 委外回料行
-- ---------------------------------------------------------------------------
create table if not exists wms.wms_outsource_back_d (
    id                      bigserial primary key,
    outsource_id            bigint not null references wms.wms_outsource_h(id) on delete cascade,
    line_no                 integer not null,
    material_id             bigint not null references mdm.mdm_material(id),
    batch_no                varchar(100) not null default '',
    expected_qty            numeric(18,4) not null,
    received_qty            numeric(18,4) not null default 0,
    bad_qty                 numeric(18,4) not null default 0,
    loss_qty                numeric(18,4) not null default 0,
    unit                    varchar(30) not null,
    inbound_line_id         bigint references wms.wms_inbound_d(id),
    note                    text,
    created_at              timestamp not null default now(),
    unique(outsource_id, line_no),
    check (expected_qty > 0),
    check (received_qty >= 0),
    check (bad_qty >= 0),
    check (loss_qty >= 0)
);
create index if not exists idx_outsource_back_outsource on wms.wms_outsource_back_d(outsource_id);
create index if not exists idx_outsource_back_material on wms.wms_outsource_back_d(material_id);
create index if not exists idx_outsource_back_inbound on wms.wms_outsource_back_d(inbound_line_id) where inbound_line_id is not null;
comment on table wms.wms_outsource_back_d is '委外回料行';

-- ---------------------------------------------------------------------------
-- 4. 盘点单头
-- ---------------------------------------------------------------------------
create table if not exists wms.wms_stocktake_h (
    id                  bigserial primary key,
    stocktake_no        varchar(50) not null unique,
    stocktake_type      varchar(30) not null default 'FULL',
    wh_id               bigint references mdm.mdm_warehouse(id),
    loc_id              bigint references mdm.mdm_location(id),
    stocktake_date      date not null,
    operator_id         bigint references sys.sys_user(id),
    doc_status          varchar(20) not null default 'DRAFT',
    remark              text,
    created_at          timestamp not null default now(),
    updated_at          timestamp not null default now(),
    check (stocktake_type in ('FULL','PARTIAL','SPOT')),
    check (doc_status in ('DRAFT','COUNTING','DIFF_CONFIRMED','COMPLETED','VOIDED'))
);
create index if not exists idx_stocktake_h_date on wms.wms_stocktake_h(stocktake_date);
create index if not exists idx_stocktake_h_wh on wms.wms_stocktake_h(wh_id);
create index if not exists idx_stocktake_h_status on wms.wms_stocktake_h(doc_status);
comment on table wms.wms_stocktake_h is '盘点单头';
comment on column wms.wms_stocktake_h.stocktake_type is 'FULL全盘/PARTIAL抽盘/SPOT动盘';

create trigger trg_wms_stocktake_h_updated_at
    before update on wms.wms_stocktake_h
    for each row execute function sys.fn_set_updated_at();

-- ---------------------------------------------------------------------------
-- 5. 盘点单行
-- ---------------------------------------------------------------------------
create table if not exists wms.wms_stocktake_d (
    id                  bigserial primary key,
    stocktake_id        bigint not null references wms.wms_stocktake_h(id) on delete cascade,
    line_no             integer not null,
    material_id         bigint not null references mdm.mdm_material(id),
    wh_id               bigint not null references mdm.mdm_warehouse(id),
    loc_id              bigint not null references mdm.mdm_location(id),
    batch_no            varchar(100) not null default '',
    stock_status        varchar(30) not null,
    book_qty            numeric(18,4) not null default 0,
    actual_qty          numeric(18,4) not null default 0,
    diff_qty            numeric(18,4) generated always as (actual_qty - book_qty) stored,
    diff_reason         varchar(100),
    adjust_status       varchar(20) not null default 'PENDING',
    note                text,
    created_at          timestamp not null default now(),
    unique(stocktake_id, line_no),
    check (book_qty >= 0),
    check (actual_qty >= 0),
    check (adjust_status in ('PENDING','CONFIRMED','ADJUSTED','SKIPPED'))
);
create index if not exists idx_stocktake_d_stocktake on wms.wms_stocktake_d(stocktake_id);
create index if not exists idx_stocktake_d_material on wms.wms_stocktake_d(material_id);
create index if not exists idx_stocktake_d_diff on wms.wms_stocktake_d(stocktake_id) where diff_qty <> 0;
comment on table wms.wms_stocktake_d is '盘点单行';
