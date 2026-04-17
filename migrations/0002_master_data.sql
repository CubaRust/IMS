-- ============================================================================
-- 0002_master_data.sql
-- 主数据:系统用户/权限 + 仓库/仓位/供应商/客户/物料 + BOM/工艺路线
-- ============================================================================

-- ---------------------------------------------------------------------------
-- 1. 系统用户与权限
-- ---------------------------------------------------------------------------
create table if not exists sys.sys_user (
    id                  bigserial primary key,
    user_code           varchar(50) not null unique,
    user_name           varchar(100) not null,
    login_name          varchar(100) not null unique,
    password_hash       varchar(255) not null,
    mobile              varchar(30),
    is_active           boolean not null default true,
    created_at          timestamp not null default now(),
    updated_at          timestamp not null default now()
);
comment on table sys.sys_user is '系统用户';

create trigger trg_sys_user_updated_at
    before update on sys.sys_user
    for each row execute function sys.fn_set_updated_at();

create table if not exists sys.sys_role (
    id                  bigserial primary key,
    role_code           varchar(50) not null unique,
    role_name           varchar(100) not null,
    is_active           boolean not null default true,
    created_at          timestamp not null default now(),
    updated_at          timestamp not null default now()
);
comment on table sys.sys_role is '角色';

create trigger trg_sys_role_updated_at
    before update on sys.sys_role
    for each row execute function sys.fn_set_updated_at();

create table if not exists sys.sys_user_role (
    id                  bigserial primary key,
    user_id             bigint not null references sys.sys_user(id) on delete cascade,
    role_id             bigint not null references sys.sys_role(id) on delete cascade,
    created_at          timestamp not null default now(),
    unique(user_id, role_id)
);
create index if not exists idx_sys_user_role_user on sys.sys_user_role(user_id);
create index if not exists idx_sys_user_role_role on sys.sys_user_role(role_id);
comment on table sys.sys_user_role is '用户角色关系';

create table if not exists sys.sys_permission (
    id                  bigserial primary key,
    perm_code           varchar(100) not null unique,
    perm_name           varchar(150) not null,
    module_code         varchar(50) not null,
    action_code         varchar(50) not null,
    created_at          timestamp not null default now()
);
create index if not exists idx_sys_permission_module on sys.sys_permission(module_code);
comment on table sys.sys_permission is '权限点';

create table if not exists sys.sys_role_permission (
    id                  bigserial primary key,
    role_id             bigint not null references sys.sys_role(id) on delete cascade,
    permission_id       bigint not null references sys.sys_permission(id) on delete cascade,
    created_at          timestamp not null default now(),
    unique(role_id, permission_id)
);
create index if not exists idx_sys_role_permission_role on sys.sys_role_permission(role_id);
comment on table sys.sys_role_permission is '角色权限关系';

-- ---------------------------------------------------------------------------
-- 2. 仓库与仓位
-- ---------------------------------------------------------------------------
create table if not exists mdm.mdm_warehouse (
    id                  bigserial primary key,
    wh_code             varchar(50) not null unique,
    wh_name             varchar(100) not null,
    wh_type             varchar(30) not null,
    is_active           boolean not null default true,
    remark              text,
    created_at          timestamp not null default now(),
    updated_at          timestamp not null default now(),
    check (wh_type in ('RAW_WH','SEMI_WH','FG_WH','BAD_WH','SCRAP_WH','TRANSIT_WH','RETURN_WH','CHECK_WH'))
);
create index if not exists idx_mdm_warehouse_type on mdm.mdm_warehouse(wh_type);
comment on table mdm.mdm_warehouse is '仓库';
comment on column mdm.mdm_warehouse.wh_type is 'RAW_WH/SEMI_WH/FG_WH/BAD_WH/SCRAP_WH/TRANSIT_WH/RETURN_WH/CHECK_WH';

create trigger trg_mdm_warehouse_updated_at
    before update on mdm.mdm_warehouse
    for each row execute function sys.fn_set_updated_at();

create table if not exists mdm.mdm_location (
    id                  bigserial primary key,
    wh_id               bigint not null references mdm.mdm_warehouse(id),
    loc_code            varchar(50) not null,
    loc_name            varchar(100) not null,
    loc_type            varchar(30) not null default 'NORMAL',
    is_active           boolean not null default true,
    remark              text,
    created_at          timestamp not null default now(),
    updated_at          timestamp not null default now(),
    unique (wh_id, loc_code),
    check (loc_type in ('NORMAL','IQC','BAD','SCRAP','TRANSIT','RETURN','HOLD'))
);
create index if not exists idx_mdm_location_wh on mdm.mdm_location(wh_id);
create index if not exists idx_mdm_location_type on mdm.mdm_location(loc_type);
comment on table mdm.mdm_location is '仓位';

create trigger trg_mdm_location_updated_at
    before update on mdm.mdm_location
    for each row execute function sys.fn_set_updated_at();

-- ---------------------------------------------------------------------------
-- 3. 供应商与客户
-- ---------------------------------------------------------------------------
create table if not exists mdm.mdm_supplier (
    id                  bigserial primary key,
    supplier_code       varchar(50) not null unique,
    supplier_name       varchar(200) not null,
    contact_name        varchar(100),
    contact_phone       varchar(50),
    address             text,
    is_active           boolean not null default true,
    remark              text,
    created_at          timestamp not null default now(),
    updated_at          timestamp not null default now()
);
create index if not exists idx_mdm_supplier_name on mdm.mdm_supplier(supplier_name);
comment on table mdm.mdm_supplier is '供应商(含委外厂商)';

create trigger trg_mdm_supplier_updated_at
    before update on mdm.mdm_supplier
    for each row execute function sys.fn_set_updated_at();

create table if not exists mdm.mdm_customer (
    id                  bigserial primary key,
    customer_code       varchar(50) not null unique,
    customer_name       varchar(200) not null,
    contact_name        varchar(100),
    contact_phone       varchar(50),
    address             text,
    is_active           boolean not null default true,
    remark              text,
    created_at          timestamp not null default now(),
    updated_at          timestamp not null default now()
);
create index if not exists idx_mdm_customer_name on mdm.mdm_customer(customer_name);
comment on table mdm.mdm_customer is '客户';

create trigger trg_mdm_customer_updated_at
    before update on mdm.mdm_customer
    for each row execute function sys.fn_set_updated_at();

-- ---------------------------------------------------------------------------
-- 4. 物料主表
-- ---------------------------------------------------------------------------
create table if not exists mdm.mdm_material (
    id                      bigserial primary key,
    material_code           varchar(100) not null unique,
    material_name           varchar(200) not null,
    short_name              varchar(100),
    material_category       varchar(30) not null,
    spec_model              varchar(200),
    brand                   varchar(100),
    unit                    varchar(30) not null,
    process_type            varchar(30),
    has_ic_flag             boolean not null default false,
    key_material_flag       boolean not null default false,
    public_material_flag    boolean not null default false,
    batch_required_flag     boolean not null default true,
    status_required_flag    boolean not null default true,
    allow_preissue_flag     boolean not null default false,
    allow_outsource_flag    boolean not null default false,
    allow_recovery_flag     boolean not null default false,
    default_wh_id           bigint references mdm.mdm_warehouse(id),
    default_loc_id          bigint references mdm.mdm_location(id),
    default_status          varchar(30),
    safety_stock            numeric(18,4) not null default 0,
    min_stock               numeric(18,4) not null default 0,
    extra_attrs             jsonb not null default '{}'::jsonb,
    is_active               boolean not null default true,
    remark                  text,
    created_at              timestamp not null default now(),
    updated_at              timestamp not null default now(),
    check (material_category in ('RAW','SEMI','FINISHED','PUBLIC','RECOVERY','SCRAP')),
    check (process_type is null or process_type in ('GG','GF','TP','ASM','FOG','OTHER')),
    check (safety_stock >= 0),
    check (min_stock >= 0)
);
create index if not exists idx_mdm_material_category on mdm.mdm_material(material_category);
create index if not exists idx_mdm_material_process_type on mdm.mdm_material(process_type);
create index if not exists idx_mdm_material_name on mdm.mdm_material(material_name);
create index if not exists idx_mdm_material_brand on mdm.mdm_material(brand);
create index if not exists idx_mdm_material_extra_attrs_gin on mdm.mdm_material using gin(extra_attrs);
comment on table mdm.mdm_material is '物料主表';
comment on column mdm.mdm_material.material_category is 'RAW/SEMI/FINISHED/PUBLIC/RECOVERY/SCRAP';
comment on column mdm.mdm_material.process_type is 'GG/GF/TP/ASM/FOG/OTHER';
comment on column mdm.mdm_material.extra_attrs is '扩展属性:客户特殊要求、工艺扩展条件等';

create trigger trg_mdm_material_updated_at
    before update on mdm.mdm_material
    for each row execute function sys.fn_set_updated_at();

-- ---------------------------------------------------------------------------
-- 5. 工艺路线
-- ---------------------------------------------------------------------------
create table if not exists mdm.mdm_route_h (
    id                  bigserial primary key,
    route_code          varchar(50) not null unique,
    route_name          varchar(200) not null,
    product_material_id bigint not null references mdm.mdm_material(id),
    is_active           boolean not null default true,
    remark              text,
    created_at          timestamp not null default now(),
    updated_at          timestamp not null default now()
);
create index if not exists idx_mdm_route_h_product on mdm.mdm_route_h(product_material_id);
comment on table mdm.mdm_route_h is '工艺路线头';

create trigger trg_mdm_route_h_updated_at
    before update on mdm.mdm_route_h
    for each row execute function sys.fn_set_updated_at();

create table if not exists mdm.mdm_route_d (
    id                  bigserial primary key,
    route_id            bigint not null references mdm.mdm_route_h(id) on delete cascade,
    step_no             integer not null,
    process_name        varchar(100) not null,
    output_material_id  bigint references mdm.mdm_material(id),
    semi_finished_flag  boolean not null default false,
    rule_json           jsonb not null default '{}'::jsonb,
    remark              text,
    created_at          timestamp not null default now(),
    unique(route_id, step_no)
);
create index if not exists idx_mdm_route_d_route on mdm.mdm_route_d(route_id);
create index if not exists idx_mdm_route_d_output on mdm.mdm_route_d(output_material_id);
comment on table mdm.mdm_route_d is '工艺路线步骤';

-- ---------------------------------------------------------------------------
-- 6. BOM
-- ---------------------------------------------------------------------------
create table if not exists mdm.mdm_bom_h (
    id                  bigserial primary key,
    bom_code            varchar(50) not null unique,
    bom_version         varchar(30) not null,
    product_material_id bigint not null references mdm.mdm_material(id),
    route_id            bigint references mdm.mdm_route_h(id),
    is_active           boolean not null default true,
    remark              text,
    created_at          timestamp not null default now(),
    updated_at          timestamp not null default now()
);
create index if not exists idx_mdm_bom_h_product on mdm.mdm_bom_h(product_material_id);
create index if not exists idx_mdm_bom_h_route on mdm.mdm_bom_h(route_id);
comment on table mdm.mdm_bom_h is 'BOM 头';

create trigger trg_mdm_bom_h_updated_at
    before update on mdm.mdm_bom_h
    for each row execute function sys.fn_set_updated_at();

create table if not exists mdm.mdm_bom_d (
    id                      bigserial primary key,
    bom_id                  bigint not null references mdm.mdm_bom_h(id) on delete cascade,
    line_no                 integer not null,
    material_id             bigint not null references mdm.mdm_material(id),
    usage_qty               numeric(18,6) not null,
    loss_rate               numeric(10,6) not null default 0,
    public_material_flag    boolean not null default false,
    extra_attrs             jsonb not null default '{}'::jsonb,
    remark                  text,
    created_at              timestamp not null default now(),
    unique(bom_id, line_no),
    check (usage_qty > 0),
    check (loss_rate >= 0)
);
create index if not exists idx_mdm_bom_d_bom on mdm.mdm_bom_d(bom_id);
create index if not exists idx_mdm_bom_d_material on mdm.mdm_bom_d(material_id);
comment on table mdm.mdm_bom_d is 'BOM 行';

-- ---------------------------------------------------------------------------
-- 7. 状态流转规则
-- ---------------------------------------------------------------------------
create table if not exists mdm.mdm_status_flow (
    id                  bigserial primary key,
    source_status       varchar(30) not null,
    target_status       varchar(30) not null,
    scene_code          varchar(50) not null,
    need_auth_flag      boolean not null default false,
    is_active           boolean not null default true,
    remark              text,
    created_at          timestamp not null default now(),
    unique(source_status, target_status, scene_code)
);
create index if not exists idx_mdm_status_flow_source on mdm.mdm_status_flow(source_status);
create index if not exists idx_mdm_status_flow_scene on mdm.mdm_status_flow(scene_code);
comment on table mdm.mdm_status_flow is '库存状态流转规则';

-- ---------------------------------------------------------------------------
-- 8. 拆解回收模板
-- ---------------------------------------------------------------------------
create table if not exists mdm.mdm_recovery_tpl_h (
    id                  bigserial primary key,
    tpl_code            varchar(50) not null unique,
    tpl_name            varchar(200) not null,
    source_material_id  bigint not null references mdm.mdm_material(id),
    is_active           boolean not null default true,
    remark              text,
    created_at          timestamp not null default now(),
    updated_at          timestamp not null default now()
);
create index if not exists idx_mdm_recovery_tpl_h_source on mdm.mdm_recovery_tpl_h(source_material_id);
comment on table mdm.mdm_recovery_tpl_h is '拆解回收模板头';

create trigger trg_mdm_recovery_tpl_h_updated_at
    before update on mdm.mdm_recovery_tpl_h
    for each row execute function sys.fn_set_updated_at();

create table if not exists mdm.mdm_recovery_tpl_d (
    id                      bigserial primary key,
    tpl_id                  bigint not null references mdm.mdm_recovery_tpl_h(id) on delete cascade,
    line_no                 integer not null,
    target_material_id      bigint references mdm.mdm_material(id),
    default_recovery_qty    numeric(18,6) not null default 0,
    target_default_status   varchar(30),
    scrap_flag              boolean not null default false,
    extra_attrs             jsonb not null default '{}'::jsonb,
    remark                  text,
    created_at              timestamp not null default now(),
    unique(tpl_id, line_no),
    check (default_recovery_qty >= 0)
);
create index if not exists idx_mdm_recovery_tpl_d_tpl on mdm.mdm_recovery_tpl_d(tpl_id);
comment on table mdm.mdm_recovery_tpl_d is '拆解回收模板行';
