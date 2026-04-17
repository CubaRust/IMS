-- ============================================================================
-- 0005_preissue.sql
-- 待入库先发 / 异常先发 / 待补入库
--
-- 核心设计:不做真负库存,用 preissue 单 + pending_qty 做异常闭环
-- ============================================================================

-- ---------------------------------------------------------------------------
-- 1. 待入库先发头
-- ---------------------------------------------------------------------------
create table if not exists wms.wms_preissue_h (
    id                  bigserial primary key,
    preissue_no         varchar(50) not null unique,
    exception_type      varchar(30) not null default 'PREISSUE',
    supplier_id         bigint references mdm.mdm_supplier(id),
    work_order_no       varchar(50),
    process_name        varchar(100),
    workshop_name       varchar(100),
    issue_date          date not null,
    operator_id         bigint references sys.sys_user(id),
    reason              text not null,
    exception_status    varchar(20) not null default 'PENDING',
    timeout_flag        boolean not null default false,
    expected_close_date date,
    extra_json          jsonb not null default '{}'::jsonb,
    remark              text,
    created_at          timestamp not null default now(),
    updated_at          timestamp not null default now(),
    check (exception_status in ('PENDING','PARTIAL','CLOSED','OVERTIME','VOIDED'))
);
create index if not exists idx_preissue_h_status on wms.wms_preissue_h(exception_status);
create index if not exists idx_preissue_h_date on wms.wms_preissue_h(issue_date);
create index if not exists idx_preissue_h_supplier on wms.wms_preissue_h(supplier_id);
create index if not exists idx_preissue_h_work_order on wms.wms_preissue_h(work_order_no) where work_order_no is not null;
create index if not exists idx_preissue_h_timeout on wms.wms_preissue_h(timeout_flag) where timeout_flag = true;
comment on table wms.wms_preissue_h is '待入库先发单头(异常先发)';
comment on column wms.wms_preissue_h.exception_status is 'PENDING/PARTIAL/CLOSED/OVERTIME/VOIDED';

create trigger trg_wms_preissue_h_updated_at
    before update on wms.wms_preissue_h
    for each row execute function sys.fn_set_updated_at();

-- ---------------------------------------------------------------------------
-- 2. 待入库先发行
-- ---------------------------------------------------------------------------
create table if not exists wms.wms_preissue_d (
    id                          bigserial primary key,
    preissue_id                 bigint not null references wms.wms_preissue_h(id) on delete cascade,
    line_no                     integer not null,
    material_id                 bigint not null references mdm.mdm_material(id),
    qty                         numeric(18,4) not null,
    filled_qty                  numeric(18,4) not null default 0,
    unfilled_qty                numeric(18,4) not null,
    expected_batch_no           varchar(100),
    target_desc                 varchar(200),
    line_status                 varchar(20) not null default 'PENDING',
    closed_by_inbound_line_id   bigint,
    note                        text,
    created_at                  timestamp not null default now(),
    unique(preissue_id, line_no),
    check (qty > 0),
    check (filled_qty >= 0),
    check (unfilled_qty >= 0),
    check (filled_qty + unfilled_qty = qty),
    check (line_status in ('PENDING','PARTIAL','CLOSED'))
);
create index if not exists idx_preissue_d_preissue on wms.wms_preissue_d(preissue_id);
create index if not exists idx_preissue_d_material on wms.wms_preissue_d(material_id);
create index if not exists idx_preissue_d_status on wms.wms_preissue_d(line_status);
create index if not exists idx_preissue_d_material_status on wms.wms_preissue_d(material_id, line_status) where line_status <> 'CLOSED';
comment on table wms.wms_preissue_d is '待入库先发行';

-- 补 0004 入库单行对 preissue_d 的外键(循环依赖只能这里补)
alter table wms.wms_inbound_d
    add constraint fk_inbound_d_preissue_line
    foreign key (related_preissue_line_id)
    references wms.wms_preissue_d(id);
