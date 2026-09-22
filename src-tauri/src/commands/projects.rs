use crate::{
    commands::auth::require_session,
    database::{backfill_product_consumptions_for_source, round_money, snapshot_for_user},
    models::{AppSnapshot, ProjectDefinitionInput},
    state::DatabaseState,
};
use chrono::Utc;
use rusqlite::params;
use uuid::Uuid;

fn validate_input(
    input: &ProjectDefinitionInput,
) -> Result<(String, i64, f64, Option<String>, f64), String> {
    let name = input.name.trim().to_string();
    if name.is_empty()
        || input.duration <= 0
        || input.duration > 1440
        || !input.price.is_finite()
        || input.price < 0.0
        || !input.consumption_quantity.is_finite()
        || input.consumption_quantity < 0.0
    {
        return Err("项目名称、时长或价格无效".to_string());
    }
    let product_id = input
        .product_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let quantity = if product_id.is_some() {
        input.consumption_quantity
    } else {
        0.0
    };
    Ok((
        name,
        input.duration,
        round_money(input.price),
        product_id,
        quantity,
    ))
}

#[tauri::command]
pub(crate) fn create_project(
    token: String,
    input: ProjectDefinitionInput,
    state: tauri::State<DatabaseState>,
) -> Result<AppSnapshot, String> {
    let user = require_session(&state, &token)?;
    let (name, duration, price, product_id, consumption_quantity) = validate_input(&input)?;
    let connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    let now = Utc::now().to_rfc3339();
    if let Some(product_id) = product_id.as_deref() {
        connection
            .query_row(
                "SELECT 1 FROM products WHERE id=?1 AND status='active'",
                params![product_id],
                |_| Ok(()),
            )
            .map_err(|_| "请选择有效的消耗产品".to_string())?;
    }
    connection
        .execute(
            "INSERT INTO projects
             (id,name,duration,price,product_id,consumption_quantity,status,created_at,updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,'active',?7,?7)",
            params![
                Uuid::new_v4().to_string(),
                name,
                duration,
                price,
                product_id,
                consumption_quantity,
                now
            ],
        )
        .map_err(|error| {
            if error.to_string().contains("UNIQUE") {
                "项目名称已存在".to_string()
            } else {
                error.to_string()
            }
        })?;
    snapshot_for_user(&connection, &user)
}

#[tauri::command]
pub(crate) fn update_project(
    token: String,
    project_id: String,
    input: ProjectDefinitionInput,
    state: tauri::State<DatabaseState>,
) -> Result<AppSnapshot, String> {
    let user = require_session(&state, &token)?;
    let (name, duration, price, product_id, consumption_quantity) = validate_input(&input)?;
    let mut connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    if let Some(product_id) = product_id.as_deref() {
        connection
            .query_row(
                "SELECT 1 FROM products WHERE id=?1 AND status='active'",
                params![product_id],
                |_| Ok(()),
            )
            .map_err(|_| "请选择有效的消耗产品".to_string())?;
    }
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    let changed = transaction
        .execute(
            "UPDATE projects SET name=?1,duration=?2,price=?3,product_id=?4,
             consumption_quantity=?5,updated_at=?6 WHERE id=?7",
            params![
                name,
                duration,
                price,
                product_id,
                consumption_quantity,
                Utc::now().to_rfc3339(),
                project_id
            ],
        )
        .map_err(|error| {
            if error.to_string().contains("UNIQUE") {
                "项目名称已存在".to_string()
            } else {
                error.to_string()
            }
        })?;
    if changed == 0 {
        return Err("项目不存在".to_string());
    }
    backfill_product_consumptions_for_source(&transaction, "project", &project_id)?;
    transaction.commit().map_err(|error| error.to_string())?;
    snapshot_for_user(&connection, &user)
}

#[tauri::command]
pub(crate) fn set_project_status(
    token: String,
    project_id: String,
    status: String,
    state: tauri::State<DatabaseState>,
) -> Result<AppSnapshot, String> {
    let user = require_session(&state, &token)?;
    if status != "active" && status != "inactive" {
        return Err("项目状态无效".to_string());
    }
    let connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    let changed = connection
        .execute(
            "UPDATE projects SET status=?1,updated_at=?2 WHERE id=?3",
            params![status, Utc::now().to_rfc3339(), project_id],
        )
        .map_err(|error| error.to_string())?;
    if changed == 0 {
        return Err("项目不存在".to_string());
    }
    snapshot_for_user(&connection, &user)
}
