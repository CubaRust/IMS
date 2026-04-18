//! outbound repo

use async_trait::async_trait;
use sqlx::{postgres::PgRow, PgPool, Postgres, Row};

use cuba_shared::{audit::AuditContext, error::AppError, types::DocStatus};

use crate::application::{
    CreateOutboundCommand, OutboundHeadView, OutboundLineView, QueryOutbounds,
};

#[async_trait]
pub trait OutboundRepository: Send + Sync {
    async fn create(
        &self,
        ctx: &AuditContext,
        cmd: &CreateOutboundCommand,
    ) -> Result<OutboundHeadView, AppError>;
    async fn get(&self, tenant_id: i64, id: i64) -> Result<OutboundHeadView, AppError>;
    async fn list(
        &self,
        tenant_id: i64,
        q: &QueryOutbounds,
    ) -> Result<Vec<OutboundHeadView>, AppError>;
    async fn update_status(
        &self,
        tenant_id: i64,
        id: i64,
        status: DocStatus,
    ) -> Result<(), AppError>;
}

pub struct PgOutboundRepository {
    pool: PgPool,
}

impl PgOutboundRepository {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl OutboundRepository for PgOutboundRepository {
    async fn create(
        &self,
        ctx: &AuditContext,
        cmd: &CreateOutboundCommand,
    ) -> Result<OutboundHeadView, AppError> {
        let mut tx = self.pool.begin().await?;

        let outbound_no: String =
            sqlx::query_scalar("select sys.fn_next_doc_no('OUTBOUND')")
                .fetch_one(&mut *tx)
                .await?;

        let id: i64 = sqlx::query_scalar(
            r#"
            insert into wms.wms_outbound_h
                (tenant_id, outbound_no, outbound_type,
                 target_object_type, target_object_id,
                 work_order_no, process_name, route_id, workshop_name,
                 wh_id, loc_id, outbound_date, operator_id, doc_status, remark)
            values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,'DRAFT',$14)
            returning id
            "#,
        )
        .bind(ctx.tenant_id)
        .bind(&outbound_no)
        .bind(&cmd.outbound_type)
        .bind(&cmd.target_object_type)
        .bind(cmd.target_object_id)
        .bind(&cmd.work_order_no)
        .bind(&cmd.process_name)
        .bind(cmd.route_id)
        .bind(&cmd.workshop_name)
        .bind(cmd.wh_id)
        .bind(cmd.loc_id)
        .bind(cmd.outbound_date)
        .bind(ctx.user_id)
        .bind(&cmd.remark)
        .fetch_one(&mut *tx)
        .await?;

        for l in &cmd.lines {
            sqlx::query(
                r#"
                insert into wms.wms_outbound_d
                    (tenant_id, outbound_id, line_no, material_id, batch_no,
                     suggest_qty, actual_qty, unit, stock_status,
                     bom_recommended_flag, public_material_flag, preissue_flag, note)
                values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)
                "#,
            )
            .bind(ctx.tenant_id)
            .bind(id)
            .bind(l.line_no)
            .bind(l.material_id)
            .bind(&l.batch_no)
            .bind(l.suggest_qty)
            .bind(l.actual_qty)
            .bind(&l.unit)
            .bind(&l.stock_status)
            .bind(l.bom_recommended_flag)
            .bind(l.public_material_flag)
            .bind(l.preissue_flag)
            .bind(&l.note)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        self.get(ctx.tenant_id, id).await
    }

    async fn get(&self, tenant_id: i64, id: i64) -> Result<OutboundHeadView, AppError> {
        let head = sqlx::query(
            r#"
            select h.id, h.outbound_no, h.outbound_type,
                   h.target_object_type, h.target_object_id,
                   h.work_order_no, h.process_name, h.route_id, h.workshop_name,
                   h.wh_id, w.wh_code, h.loc_id, l.loc_code,
                   h.outbound_date, h.operator_id, h.doc_status, h.remark,
                   h.created_at, h.updated_at
              from wms.wms_outbound_h h
              left join mdm.mdm_warehouse w on w.id = h.wh_id
              left join mdm.mdm_location  l on l.id = h.loc_id
             where h.id = $1 and h.tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::not_found(format!("出库单 id={id} 不存在")))?;

        let lines = sqlx::query(
            r#"
            select d.id, d.line_no, d.material_id, m.material_code,
                   d.batch_no, d.suggest_qty, d.actual_qty, d.unit, d.stock_status,
                   d.bom_recommended_flag, d.public_material_flag, d.preissue_flag, d.note
              from wms.wms_outbound_d d
              left join mdm.mdm_material m on m.id = d.material_id
             where d.outbound_id = $1 and d.tenant_id = $2
             order by d.line_no
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(row_to_line)
        .collect();

        Ok(row_to_head(head, lines))
    }

    async fn list(
        &self,
        tenant_id: i64,
        q: &QueryOutbounds,
    ) -> Result<Vec<OutboundHeadView>, AppError> {
        let mut qb = sqlx::QueryBuilder::<Postgres>::new(
            r#"
            select h.id, h.outbound_no, h.outbound_type,
                   h.target_object_type, h.target_object_id,
                   h.work_order_no, h.process_name, h.route_id, h.workshop_name,
                   h.wh_id, w.wh_code, h.loc_id, l.loc_code,
                   h.outbound_date, h.operator_id, h.doc_status, h.remark,
                   h.created_at, h.updated_at
              from wms.wms_outbound_h h
              left join mdm.mdm_warehouse w on w.id = h.wh_id
              left join mdm.mdm_location  l on l.id = h.loc_id
             where h.tenant_id =
            "#,
        );
        qb.push_bind(tenant_id);
        if let Some(no) = &q.outbound_no {
            qb.push(" and h.outbound_no = ").push_bind(no.clone());
        }
        if let Some(t) = &q.outbound_type {
            qb.push(" and h.outbound_type = ").push_bind(t.clone());
        }
        if let Some(wo) = &q.work_order_no {
            qb.push(" and h.work_order_no = ").push_bind(wo.clone());
        }
        if let Some(s) = &q.doc_status {
            qb.push(" and h.doc_status = ").push_bind(s.clone());
        }
        if let Some(from) = q.date_from {
            qb.push(" and h.outbound_date >= ").push_bind(from);
        }
        if let Some(to) = q.date_to {
            qb.push(" and h.outbound_date <= ").push_bind(to);
        }
        qb.push(" order by h.outbound_date desc, h.id desc limit 500");
        let rows = qb.build().fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|r| row_to_head(r, vec![])).collect())
    }

    async fn update_status(
        &self,
        tenant_id: i64,
        id: i64,
        status: DocStatus,
    ) -> Result<(), AppError> {
        sqlx::query(
            "update wms.wms_outbound_h set doc_status = $1 where id = $2 and tenant_id = $3",
        )
        .bind(status.as_str())
        .bind(id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

fn row_to_head(row: PgRow, lines: Vec<OutboundLineView>) -> OutboundHeadView {
    OutboundHeadView {
        id: row.get("id"),
        outbound_no: row.get("outbound_no"),
        outbound_type: row.get("outbound_type"),
        target_object_type: row.get("target_object_type"),
        target_object_id: row.get("target_object_id"),
        work_order_no: row.get("work_order_no"),
        process_name: row.get("process_name"),
        route_id: row.get("route_id"),
        workshop_name: row.get("workshop_name"),
        wh_id: row.get("wh_id"),
        wh_code: row.get("wh_code"),
        loc_id: row.get("loc_id"),
        loc_code: row.get("loc_code"),
        outbound_date: row.get("outbound_date"),
        operator_id: row.get("operator_id"),
        doc_status: row.get("doc_status"),
        remark: row.get("remark"),
        lines,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn row_to_line(row: PgRow) -> OutboundLineView {
    OutboundLineView {
        id: row.get("id"),
        line_no: row.get("line_no"),
        material_id: row.get("material_id"),
        material_code: row.get("material_code"),
        batch_no: row.get("batch_no"),
        suggest_qty: row.get("suggest_qty"),
        actual_qty: row.get("actual_qty"),
        unit: row.get("unit"),
        stock_status: row.get("stock_status"),
        bom_recommended_flag: row.get("bom_recommended_flag"),
        public_material_flag: row.get("public_material_flag"),
        preissue_flag: row.get("preissue_flag"),
        note: row.get("note"),
    }
}
