-- ============================================================================
-- 0001_init.sql
-- 初始化:扩展、schema、基础表、通用函数、触发器
--
-- 执行顺序:
--   1. 扩展
--   2. schema
--   3. 基础表(编码规则、数据字典)
--   4. 通用函数(updated_at 触发器函数、单据号生成)
--   5. 基础表的触发器
-- ============================================================================

-- ---------------------------------------------------------------------------
-- 1. 扩展
-- ---------------------------------------------------------------------------
create extension if not exists "uuid-ossp";
create extension if not exists "pgcrypto";

-- ---------------------------------------------------------------------------
-- 2. Schema
-- ---------------------------------------------------------------------------
create schema if not exists sys;
create schema if not exists mdm;
create schema if not exists wms;
create schema if not exists rpt;

comment on schema sys is '系统管理';
comment on schema mdm is '主数据';
comment on schema wms is '仓库业务';
comment on schema rpt is '报表视图';

-- ---------------------------------------------------------------------------
-- 3. 单据编码规则表
-- ---------------------------------------------------------------------------
create table if not exists sys.sys_doc_no_rule (
    id                  bigserial primary key,
    doc_type            varchar(50) not null unique,
    doc_prefix          varchar(20) not null,
    date_pattern        varchar(20) not null default 'YYYYMMDD',
    seq_length          integer not null default 4,
    current_date_str    varchar(20),
    current_seq         integer not null default 0,
    updated_at          timestamp not null default now()
);
comment on table sys.sys_doc_no_rule is '单据编码规则';

-- ---------------------------------------------------------------------------
-- 4. 数据字典表
-- ---------------------------------------------------------------------------
create table if not exists sys.sys_dict (
    id              bigserial primary key,
    dict_type       varchar(50) not null,
    dict_key        varchar(100) not null,
    dict_value      varchar(200) not null,
    dict_order      integer not null default 0,
    is_active       boolean not null default true,
    remark          text,
    created_at      timestamp not null default now(),
    updated_at      timestamp not null default now(),
    unique(dict_type, dict_key)
);
create index if not exists idx_sys_dict_type on sys.sys_dict(dict_type);
comment on table sys.sys_dict is '数据字典(枚举值展示)';

-- ---------------------------------------------------------------------------
-- 5. 公共函数:自动更新 updated_at
-- ---------------------------------------------------------------------------
create or replace function sys.fn_set_updated_at()
returns trigger as $$
begin
    new.updated_at = now();
    return new;
end;
$$ language plpgsql;

comment on function sys.fn_set_updated_at() is '通用触发器:自动更新 updated_at 字段';

-- ---------------------------------------------------------------------------
-- 6. 公共函数:生成单据号
-- 规则:prefix + yyyyMMdd + 自增序号(补零)
-- 调用:select sys.fn_next_doc_no('INBOUND');
-- 返回示例:INB202604170001
-- ---------------------------------------------------------------------------
create or replace function sys.fn_next_doc_no(p_doc_type varchar)
returns varchar as $$
declare
    v_prefix        varchar;
    v_date_pattern  varchar;
    v_seq_length    integer;
    v_date_str      varchar;
    v_current_date  varchar;
    v_current_seq   integer;
    v_result        varchar;
begin
    select doc_prefix, date_pattern, seq_length, current_date_str, current_seq
      into v_prefix, v_date_pattern, v_seq_length, v_current_date, v_current_seq
      from sys.sys_doc_no_rule
     where doc_type = p_doc_type
     for update;

    if not found then
        raise exception '单据类型 [%] 未定义编码规则', p_doc_type;
    end if;

    v_date_str := to_char(now(), v_date_pattern);

    if v_current_date is distinct from v_date_str then
        v_current_seq := 1;
    else
        v_current_seq := v_current_seq + 1;
    end if;

    update sys.sys_doc_no_rule
       set current_date_str = v_date_str,
           current_seq      = v_current_seq,
           updated_at       = now()
     where doc_type = p_doc_type;

    v_result := v_prefix || v_date_str || lpad(v_current_seq::text, v_seq_length, '0');
    return v_result;
end;
$$ language plpgsql;

comment on function sys.fn_next_doc_no(varchar) is '单据号生成:prefix + date + 自增序号';

-- ---------------------------------------------------------------------------
-- 7. 基础表触发器
-- ---------------------------------------------------------------------------
drop trigger if exists trg_sys_dict_updated_at on sys.sys_dict;
create trigger trg_sys_dict_updated_at
    before update on sys.sys_dict
    for each row execute function sys.fn_set_updated_at();
