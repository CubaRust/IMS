-- ============================================================================
-- 0003_inventory.sql
-- 库存核心:余额表 + 事务头 + 事务行
-- 重点:所有库存变化必须走 wms_inventory_txn,余额表由事务驱动更新
-- ============================================================================

-- ---------------------------------------------------------------------------
-- 1. 库存余额表
--
-- 字段说明:
--   book_qty       账面数量
--   available_qty  可用数量
--   occupied_qty   占用数量
--   bad_qty        不良数量
--   scrap_qty      报废数量
--   pending_qty    异常先发占用(对应 PREISSUE_PENDING)
--
-- 注意:"待入库先发"不做真负库存,而是通过 pending_qty + 异常单闭环
-- ---------------------------------------------------------------------------
create table if not exists wms.wms_inventory_balance (
    id                  bigserial primary key,
    material_id         bigint not null references mdm.mdm_material(id),
    wh_id               bigint not null references mdm.mdm_warehouse(id),
    loc_id              bigint not null references mdm.mdm_location(id),
    batch_no            varchar(100) not null default '',
    stock_status        varchar(30) not null,
    book_qty            numeric(18,4) not null default 0,
    available_qty       numeric(18,4) not null default 0,
    occupied_qty        numeric(18,4) not null default 0,
    bad_qty             numeric(18,4) not null default 0,
    scrap_qty           numeric(18,4) not null default 0,
    pending_qty         numeric(18,4) not null default 0,
    updated_at          timestamp not null default now(),
    unique(material_id, wh_id, loc_id, batch_no, stock_status),
    check (book_qty >= 0 or stock_status = 'PREISSUE_PENDING'),
    check (available_qty >= 0),
    check (occupied_qty >= 0),
    check (bad_qty >= 0),
    check (scrap_qty >= 0),
    check (pending_qty >= 0)
);
create index if not exists idx_inv_balance_material on wms.wms_inventory_balance(material_id);
create index if not exists idx_inv_balance_wh_loc on wms.wms_inventory_balance(wh_id, loc_id);
create index if not exists idx_inv_balance_status on wms.wms_inventory_balance(stock_status);
create index if not exists idx_inv_balance_batch on wms.wms_inventory_balance(batch_no) where batch_no <> '';
comment on table wms.wms_inventory_balance is '库存余额(按物料/仓/位/批次/状态汇总)';
comment on column wms.wms_inventory_balance.stock_status is 'TO_CHECK/QUALIFIED/BAD/FROZEN/IN_PROCESS/OUTSOURCE/PREISSUE_PENDING 等';
comment on column wms.wms_inventory_balance.pending_qty is '异常先发占用数量(不计入可用)';

create trigger trg_wms_inventory_balance_updated_at
    before update on wms.wms_inventory_balance
    for each row execute function sys.fn_set_updated_at();

-- ---------------------------------------------------------------------------
-- 2. 库存事务头
-- ---------------------------------------------------------------------------
create table if not exists wms.wms_inventory_txn_h (
    id                  bigserial primary key,
    txn_no              varchar(50) not null unique,
    txn_type            varchar(30) not null,
    scene_code          varchar(50) not null,
    scene_name          varchar(100),
    doc_type            varchar(50) not null,
    doc_no              varchar(50) not null,
    source_object_type  varchar(50),
    source_object_id    bigint,
    target_object_type  varchar(50),
    target_object_id    bigint,
    source_wh_id        bigint references mdm.mdm_warehouse(id),
    source_loc_id       bigint references mdm.mdm_location(id),
    target_wh_id        bigint references mdm.mdm_warehouse(id),
    target_loc_id       bigint references mdm.mdm_location(id),
    source_status       varchar(30),
    target_status       varchar(30),
    is_exception        boolean not null default false,
    exception_type      varchar(50),
    operator_id         bigint references sys.sys_user(id),
    related_doc_no      varchar(50),
    snapshot_json       jsonb not null default '{}'::jsonb,
    remark              text,
    operate_time        timestamp not null default now(),
    check (txn_type in ('IN','OUT','TRANSFER','CONVERT','RESERVE','RELEASE'))
);
create index if not exists idx_inv_txn_h_doc_no on wms.wms_inventory_txn_h(doc_no);
create index if not exists idx_inv_txn_h_doc_type_no on wms.wms_inventory_txn_h(doc_type, doc_no);
create index if not exists idx_inv_txn_h_scene_time on wms.wms_inventory_txn_h(scene_code, operate_time);
create index if not exists idx_inv_txn_h_operate_time on wms.wms_inventory_txn_h(operate_time);
create index if not exists idx_inv_txn_h_exception on wms.wms_inventory_txn_h(is_exception) where is_exception = true;
comment on table wms.wms_inventory_txn_h is '库存事务头(所有库存变化的唯一入口)';
comment on column wms.wms_inventory_txn_h.txn_type is 'IN/OUT/TRANSFER/CONVERT/RESERVE/RELEASE';
comment on column wms.wms_inventory_txn_h.snapshot_json is '提交瞬间业务快照';

-- ---------------------------------------------------------------------------
-- 3. 库存事务行
-- ---------------------------------------------------------------------------
create table if not exists wms.wms_inventory_txn_d (
    id                      bigserial primary key,
    txn_id                  bigint not null references wms.wms_inventory_txn_h(id) on delete cascade,
    line_no                 integer not null,
    material_id             bigint not null references mdm.mdm_material(id),
    batch_no                varchar(100) not null default '',
    qty                     numeric(18,4) not null,
    unit                    varchar(30) not null,
    io_flag                 varchar(10) not null,
    source_material_id      bigint references mdm.mdm_material(id),
    target_material_id      bigint references mdm.mdm_material(id),
    stock_status            varchar(30),
    status_change_flag      boolean not null default false,
    location_change_flag    boolean not null default false,
    item_change_flag        boolean not null default false,
    recoverable_flag        boolean not null default false,
    scrap_flag              boolean not null default false,
    note                    text,
    created_at              timestamp not null default now(),
    unique(txn_id, line_no),
    check (qty > 0),
    check (io_flag in ('IN','OUT'))
);
create index if not exists idx_inv_txn_d_txn on wms.wms_inventory_txn_d(txn_id);
create index if not exists idx_inv_txn_d_material_batch on wms.wms_inventory_txn_d(material_id, batch_no);
create index if not exists idx_inv_txn_d_material_time on wms.wms_inventory_txn_d(material_id, created_at);
comment on table wms.wms_inventory_txn_d is '库存事务行(双边流水)';
