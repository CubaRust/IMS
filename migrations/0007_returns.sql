-- ============================================================================
-- 0007_returns.sql
-- 报废 + 客户退货 + 供应商退货
--
-- 关键规则:
--   - 客户退货默认进 CUSTOMER_RETURN_PENDING 待判定
--   - 供应商退货用"退供出库单",不要用红字入库冲销
-- ============================================================================

-- ---------------------------------------------------------------------------
-- 1. 报废单头
-- ---------------------------------------------------------------------------
create table if not exists wms.wms_scrap_h (
    id                  bigserial primary key,
    scrap_no            varchar(50) not null unique,
    scrap_source        varchar(30) not null,
    source_doc_type     varchar(50),
    source_doc_no       varchar(50),
    scrap_date          date not null,
    operator_id         bigint references sys.sys_user(id),
    doc_status          varchar(20) not null default 'DRAFT',
    remark              text,
    created_at          timestamp not null default now(),
    updated_at          timestamp not null default now(),
    check (scrap_source in ('IQC_BAD','PROD_BAD','RECOVERY_LEFTOVER','CUSTOMER_RETURN_BAD','STOCKTAKE_DAMAGE','OTHER')),
    check (doc_status in ('DRAFT','SUBMITTED','COMPLETED','VOIDED'))
);
create index if not exists idx_scrap_h_date on wms.wms_scrap_h(scrap_date);
create index if not exists idx_scrap_h_source on wms.wms_scrap_h(scrap_source);
create index if not exists idx_scrap_h_status on wms.wms_scrap_h(doc_status);
comment on table wms.wms_scrap_h is '报废单头';

create trigger trg_wms_scrap_h_updated_at
    before update on wms.wms_scrap_h
    for each row execute function sys.fn_set_updated_at();

-- ---------------------------------------------------------------------------
-- 2. 报废单行
-- ---------------------------------------------------------------------------
create table if not exists wms.wms_scrap_d (
    id                  bigserial primary key,
    scrap_id            bigint not null references wms.wms_scrap_h(id) on delete cascade,
    line_no             integer not null,
    material_id         bigint references mdm.mdm_material(id),
    batch_no            varchar(100) not null default '',
    qty                 numeric(18,4) not null,
    unit                varchar(30) not null,
    stock_status        varchar(30),
    scrap_reason        varchar(50) not null,
    note                text,
    unique(scrap_id, line_no),
    check (qty > 0)
);
create index if not exists idx_scrap_d_scrap on wms.wms_scrap_d(scrap_id);
create index if not exists idx_scrap_d_material on wms.wms_scrap_d(material_id);
comment on table wms.wms_scrap_d is '报废单行';

-- ---------------------------------------------------------------------------
-- 3. 客户退货单头
-- ---------------------------------------------------------------------------
create table if not exists wms.wms_customer_return_h (
    id                  bigserial primary key,
    customer_return_no  varchar(50) not null unique,
    customer_id         bigint not null references mdm.mdm_customer(id),
    source_sales_no     varchar(50),
    return_date         date not null,
    wh_id               bigint not null references mdm.mdm_warehouse(id),
    operator_id         bigint references sys.sys_user(id),
    return_reason       text not null,
    doc_status          varchar(20) not null default 'DRAFT',
    remark              text,
    created_at          timestamp not null default now(),
    updated_at          timestamp not null default now(),
    check (doc_status in ('DRAFT','SUBMITTED','COMPLETED','VOIDED'))
);
create index if not exists idx_customer_return_h_date on wms.wms_customer_return_h(return_date);
create index if not exists idx_customer_return_h_customer on wms.wms_customer_return_h(customer_id);
create index if not exists idx_customer_return_h_status on wms.wms_customer_return_h(doc_status);
create index if not exists idx_customer_return_h_sales on wms.wms_customer_return_h(source_sales_no) where source_sales_no is not null;
comment on table wms.wms_customer_return_h is '客户退货单头';

create trigger trg_wms_customer_return_h_updated_at
    before update on wms.wms_customer_return_h
    for each row execute function sys.fn_set_updated_at();

-- ---------------------------------------------------------------------------
-- 4. 客户退货单行
-- ---------------------------------------------------------------------------
create table if not exists wms.wms_customer_return_d (
    id                  bigserial primary key,
    customer_return_id  bigint not null references wms.wms_customer_return_h(id) on delete cascade,
    line_no             integer not null,
    material_id         bigint not null references mdm.mdm_material(id),
    batch_no            varchar(100) not null default '',
    source_sales_qty    numeric(18,4),
    return_qty          numeric(18,4) not null,
    unit                varchar(30) not null,
    stock_status        varchar(30) not null default 'CUSTOMER_RETURN_PENDING',
    note                text,
    unique(customer_return_id, line_no),
    check (return_qty > 0)
);
create index if not exists idx_customer_return_d_h on wms.wms_customer_return_d(customer_return_id);
create index if not exists idx_customer_return_d_material on wms.wms_customer_return_d(material_id);
comment on table wms.wms_customer_return_d is '客户退货单行';
comment on column wms.wms_customer_return_d.stock_status is '默认 CUSTOMER_RETURN_PENDING,判定后流转';

-- ---------------------------------------------------------------------------
-- 5. 客户退货判定
-- ---------------------------------------------------------------------------
create table if not exists wms.wms_customer_return_judge (
    id                      bigserial primary key,
    customer_return_id      bigint not null references wms.wms_customer_return_h(id),
    customer_return_line_id bigint not null references wms.wms_customer_return_d(id),
    judge_result            varchar(30) not null,
    judge_qty               numeric(18,4) not null,
    judge_reason            text not null,
    judge_user_id           bigint references sys.sys_user(id),
    judge_time              timestamp not null default now(),
    check (judge_result in ('QUALIFIED','BAD','TO_DISMANTLE','SCRAPPED')),
    check (judge_qty > 0)
);
create index if not exists idx_cr_judge_h on wms.wms_customer_return_judge(customer_return_id);
create index if not exists idx_cr_judge_line on wms.wms_customer_return_judge(customer_return_line_id);
create index if not exists idx_cr_judge_time on wms.wms_customer_return_judge(judge_time);
comment on table wms.wms_customer_return_judge is '客户退货判定';

-- ---------------------------------------------------------------------------
-- 6. 供应商退货单头
-- ---------------------------------------------------------------------------
create table if not exists wms.wms_supplier_return_h (
    id                  bigserial primary key,
    supplier_return_no  varchar(50) not null unique,
    supplier_id         bigint not null references mdm.mdm_supplier(id),
    return_date         date not null,
    operator_id         bigint references sys.sys_user(id),
    return_reason       text not null,
    doc_status          varchar(20) not null default 'DRAFT',
    remark              text,
    created_at          timestamp not null default now(),
    updated_at          timestamp not null default now(),
    check (doc_status in ('DRAFT','SUBMITTED','COMPLETED','VOIDED'))
);
create index if not exists idx_supplier_return_h_date on wms.wms_supplier_return_h(return_date);
create index if not exists idx_supplier_return_h_supplier on wms.wms_supplier_return_h(supplier_id);
create index if not exists idx_supplier_return_h_status on wms.wms_supplier_return_h(doc_status);
comment on table wms.wms_supplier_return_h is '供应商退货单头(退供出库)';

create trigger trg_wms_supplier_return_h_updated_at
    before update on wms.wms_supplier_return_h
    for each row execute function sys.fn_set_updated_at();

-- ---------------------------------------------------------------------------
-- 7. 供应商退货单行
-- ---------------------------------------------------------------------------
create table if not exists wms.wms_supplier_return_d (
    id                      bigserial primary key,
    supplier_return_id      bigint not null references wms.wms_supplier_return_h(id) on delete cascade,
    line_no                 integer not null,
    material_id             bigint not null references mdm.mdm_material(id),
    batch_no                varchar(100) not null default '',
    source_inbound_line_id  bigint references wms.wms_inbound_d(id),
    return_qty              numeric(18,4) not null,
    unit                    varchar(30) not null,
    quality_result          varchar(30),
    note                    text,
    unique(supplier_return_id, line_no),
    check (return_qty > 0)
);
create index if not exists idx_supplier_return_d_h on wms.wms_supplier_return_d(supplier_return_id);
create index if not exists idx_supplier_return_d_material on wms.wms_supplier_return_d(material_id);
create index if not exists idx_supplier_return_d_src on wms.wms_supplier_return_d(source_inbound_line_id) where source_inbound_line_id is not null;
comment on table wms.wms_supplier_return_d is '供应商退货单行';
