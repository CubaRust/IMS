//! material repo

use async_trait::async_trait;
use sqlx::{postgres::PgRow, PgPool, Postgres, Row};

use cuba_shared::{
    error::AppError,
    pagination::{PageQuery, PageResponse},
};

use crate::application::material::{
    CreateMaterialCommand, MaterialView, QueryMaterials, UpdateMaterialCommand,
};

#[async_trait]
pub trait MaterialRepository: Send + Sync {
    async fn create(&self, cmd: &CreateMaterialCommand) -> Result<MaterialView, AppError>;
    async fn update(&self, id: i64, cmd: &UpdateMaterialCommand) -> Result<MaterialView, AppError>;
    async fn get(&self, id: i64) -> Result<MaterialView, AppError>;
    async fn list(&self, q: &QueryMaterials) -> Result<PageResponse<MaterialView>, AppError>;
}

pub struct PgMaterialRepository {
    pool: PgPool,
}

impl PgMaterialRepository {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MaterialRepository for PgMaterialRepository {
    async fn create(&self, cmd: &CreateMaterialCommand) -> Result<MaterialView, AppError> {
        let extra = cmd.extra_attrs.clone().unwrap_or_else(|| serde_json::json!({}));
        let id: i64 = sqlx::query_scalar(
            r#"
            insert into mdm.mdm_material
                (material_code, material_name, short_name, material_category,
                 spec_model, brand, unit, process_type,
                 has_ic_flag, key_material_flag, public_material_flag,
                 batch_required_flag, status_required_flag,
                 allow_preissue_flag, allow_outsource_flag, allow_recovery_flag,
                 default_wh_id, default_loc_id, default_status,
                 safety_stock, min_stock, extra_attrs, remark)
            values ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23)
            returning id
            "#,
        )
        .bind(&cmd.material_code)
        .bind(&cmd.material_name)
        .bind(&cmd.short_name)
        .bind(&cmd.material_category)
        .bind(&cmd.spec_model)
        .bind(&cmd.brand)
        .bind(&cmd.unit)
        .bind(&cmd.process_type)
        .bind(cmd.has_ic_flag)
        .bind(cmd.key_material_flag)
        .bind(cmd.public_material_flag)
        .bind(cmd.batch_required_flag)
        .bind(cmd.status_required_flag)
        .bind(cmd.allow_preissue_flag)
        .bind(cmd.allow_outsource_flag)
        .bind(cmd.allow_recovery_flag)
        .bind(cmd.default_wh_id)
        .bind(cmd.default_loc_id)
        .bind(&cmd.default_status)
        .bind(cmd.safety_stock)
        .bind(cmd.min_stock)
        .bind(&extra)
        .bind(&cmd.remark)
        .fetch_one(&self.pool)
        .await
        .map_err(super::map_unique_err)?;

        self.get(id).await
    }

    async fn update(
        &self,
        id: i64,
        cmd: &UpdateMaterialCommand,
    ) -> Result<MaterialView, AppError> {
        let extra = cmd.extra_attrs.clone().unwrap_or_else(|| serde_json::json!({}));
        let rows = sqlx::query(
            r#"
            update mdm.mdm_material set
                material_name = $2,
                short_name = $3,
                material_category = $4,
                spec_model = $5,
                brand = $6,
                unit = $7,
                process_type = $8,
                has_ic_flag = $9,
                key_material_flag = $10,
                public_material_flag = $11,
                batch_required_flag = $12,
                status_required_flag = $13,
                allow_preissue_flag = $14,
                allow_outsource_flag = $15,
                allow_recovery_flag = $16,
                default_wh_id = $17,
                default_loc_id = $18,
                default_status = $19,
                safety_stock = $20,
                min_stock = $21,
                extra_attrs = $22,
                is_active = $23,
                remark = $24
             where id = $1
            "#,
        )
        .bind(id)
        .bind(&cmd.material_name)
        .bind(&cmd.short_name)
        .bind(&cmd.material_category)
        .bind(&cmd.spec_model)
        .bind(&cmd.brand)
        .bind(&cmd.unit)
        .bind(&cmd.process_type)
        .bind(cmd.has_ic_flag)
        .bind(cmd.key_material_flag)
        .bind(cmd.public_material_flag)
        .bind(cmd.batch_required_flag)
        .bind(cmd.status_required_flag)
        .bind(cmd.allow_preissue_flag)
        .bind(cmd.allow_outsource_flag)
        .bind(cmd.allow_recovery_flag)
        .bind(cmd.default_wh_id)
        .bind(cmd.default_loc_id)
        .bind(&cmd.default_status)
        .bind(cmd.safety_stock)
        .bind(cmd.min_stock)
        .bind(&extra)
        .bind(cmd.is_active)
        .bind(&cmd.remark)
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows == 0 {
            return Err(AppError::not_found(format!("物料 id={id} 不存在")));
        }
        self.get(id).await
    }

    async fn get(&self, id: i64) -> Result<MaterialView, AppError> {
        let row = sqlx::query(SELECT_MATERIAL)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::not_found(format!("物料 id={id} 不存在")))?;
        Ok(row_to_view(row))
    }

    async fn list(&self, q: &QueryMaterials) -> Result<PageResponse<MaterialView>, AppError> {
        let page = PageQuery { page: q.page, size: q.size }.normalize();

        // count
        let total = {
            let mut cq = sqlx::QueryBuilder::<Postgres>::new(
                "select count(*) from mdm.mdm_material where 1 = 1",
            );
            push_filters(&mut cq, q);
            cq.build_query_scalar::<i64>()
                .fetch_one(&self.pool)
                .await
                .unwrap_or(0)
        };

        let mut qb = sqlx::QueryBuilder::<Postgres>::new(BASE_LIST_SQL);
        qb.push(" where 1 = 1");
        push_filters(&mut qb, q);
        qb.push(" order by material_code ");
        qb.push(" limit ").push_bind(page.limit());
        qb.push(" offset ").push_bind(page.offset());

        let rows = qb.build().fetch_all(&self.pool).await?;
        let items = rows.into_iter().map(row_to_view).collect();
        Ok(PageResponse::new(page, total, items))
    }
}

fn push_filters<'a>(qb: &mut sqlx::QueryBuilder<'a, Postgres>, q: &'a QueryMaterials) {
    if let Some(kw) = &q.keyword {
        qb.push(" and (material_code ilike ")
            .push_bind(format!("%{kw}%"))
            .push(" or material_name ilike ")
            .push_bind(format!("%{kw}%"))
            .push(")");
    }
    if let Some(cat) = &q.material_category {
        qb.push(" and material_category = ").push_bind(cat.clone());
    }
    if let Some(pt) = &q.process_type {
        qb.push(" and process_type = ").push_bind(pt.clone());
    }
    if let Some(br) = &q.brand {
        qb.push(" and brand = ").push_bind(br.clone());
    }
    if let Some(active) = q.is_active {
        qb.push(" and is_active = ").push_bind(active);
    }
}

const SELECT_MATERIAL: &str = r#"
    select id, material_code, material_name, short_name, material_category,
           spec_model, brand, unit, process_type,
           has_ic_flag, key_material_flag, public_material_flag,
           batch_required_flag, status_required_flag,
           allow_preissue_flag, allow_outsource_flag, allow_recovery_flag,
           default_wh_id, default_loc_id, default_status,
           safety_stock, min_stock, extra_attrs, is_active, remark,
           created_at, updated_at
      from mdm.mdm_material
     where id = $1
"#;

const BASE_LIST_SQL: &str = r#"
    select id, material_code, material_name, short_name, material_category,
           spec_model, brand, unit, process_type,
           has_ic_flag, key_material_flag, public_material_flag,
           batch_required_flag, status_required_flag,
           allow_preissue_flag, allow_outsource_flag, allow_recovery_flag,
           default_wh_id, default_loc_id, default_status,
           safety_stock, min_stock, extra_attrs, is_active, remark,
           created_at, updated_at
      from mdm.mdm_material
"#;

fn row_to_view(row: PgRow) -> MaterialView {
    MaterialView {
        id: row.get("id"),
        material_code: row.get("material_code"),
        material_name: row.get("material_name"),
        short_name: row.get("short_name"),
        material_category: row.get("material_category"),
        spec_model: row.get("spec_model"),
        brand: row.get("brand"),
        unit: row.get("unit"),
        process_type: row.get("process_type"),
        has_ic_flag: row.get("has_ic_flag"),
        key_material_flag: row.get("key_material_flag"),
        public_material_flag: row.get("public_material_flag"),
        batch_required_flag: row.get("batch_required_flag"),
        status_required_flag: row.get("status_required_flag"),
        allow_preissue_flag: row.get("allow_preissue_flag"),
        allow_outsource_flag: row.get("allow_outsource_flag"),
        allow_recovery_flag: row.get("allow_recovery_flag"),
        default_wh_id: row.get("default_wh_id"),
        default_loc_id: row.get("default_loc_id"),
        default_status: row.get("default_status"),
        safety_stock: row.get("safety_stock"),
        min_stock: row.get("min_stock"),
        extra_attrs: row.get("extra_attrs"),
        is_active: row.get("is_active"),
        remark: row.get("remark"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}
