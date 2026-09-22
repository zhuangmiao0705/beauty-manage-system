use crate::{
    commands::auth::require_manager,
    database::{round_money, snapshot_for_user},
    models::{AppSnapshot, ProductInput, SupplyPurchaseInput},
    state::DatabaseState,
};
use chrono::Utc;
use rusqlite::params;
use uuid::Uuid;

fn validate_product(input: &ProductInput) -> Result<(String, f64, f64), String> {
    let name = input.name.trim().to_string();
    if name.is_empty()
        || !input.unit_price.is_finite()
        || input.unit_price < 0.0
        || !input.stock.is_finite()
        || input.stock < 0.0
    {
        return Err("产品名称、单价或库存无效".to_string());
    }
    Ok((name, round_money(input.unit_price), input.stock))
}

#[tauri::command]
pub(crate) fn create_product(
    token: String,
    input: ProductInput,
    state: tauri::State<DatabaseState>,
) -> Result<AppSnapshot, String> {
    let manager = require_manager(&state, &token)?;
    let (name, unit_price, stock) = validate_product(&input)?;
    let connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    let now = Utc::now().to_rfc3339();
    connection
        .execute(
            "INSERT INTO products
             (id,name,unit_price,initial_stock,status,created_at,updated_at)
             VALUES (?1,?2,?3,?4,'active',?5,?5)",
            params![Uuid::new_v4().to_string(), name, unit_price, stock, now],
        )
        .map_err(|error| {
            if error.to_string().contains("UNIQUE") {
                "产品名称已存在".to_string()
            } else {
                error.to_string()
            }
        })?;
    snapshot_for_user(&connection, &manager)
}

#[tauri::command]
pub(crate) fn update_product(
    token: String,
    product_id: String,
    input: ProductInput,
    state: tauri::State<DatabaseState>,
) -> Result<AppSnapshot, String> {
    let manager = require_manager(&state, &token)?;
    let (name, unit_price, stock) = validate_product(&input)?;
    let mut connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    let current_stock = transaction
        .query_row(
            "SELECT p.initial_stock
                    + COALESCE((SELECT SUM(e.quantity) FROM product_stock_entries e
                                WHERE e.product_id=p.id),0)
                    + COALESCE((SELECT SUM(a.quantity) FROM product_stock_adjustments a
                                WHERE a.product_id=p.id),0)
                    - COALESCE((SELECT SUM(c.quantity) FROM product_consumptions c
                                WHERE c.product_id=p.id AND c.status='active'),0)
             FROM products p WHERE p.id=?1",
            params![product_id],
            |row| row.get::<_, f64>(0),
        )
        .map_err(|_| "产品不存在".to_string())?;
    let now = Utc::now().to_rfc3339();
    let changed = transaction
        .execute(
            "UPDATE products SET name=?1,unit_price=?2,updated_at=?3 WHERE id=?4",
            params![name, unit_price, now, product_id],
        )
        .map_err(|error| {
            if error.to_string().contains("UNIQUE") {
                "产品名称已存在".to_string()
            } else {
                error.to_string()
            }
        })?;
    if changed == 0 {
        return Err("产品不存在".to_string());
    }
    let adjustment = stock - current_stock;
    if adjustment.abs() > f64::EPSILON {
        transaction
            .execute(
                "INSERT INTO product_stock_adjustments(id,product_id,quantity,created_at,note)
                 VALUES (?1,?2,?3,?4,'编辑产品校正库存')",
                params![Uuid::new_v4().to_string(), product_id, adjustment, now],
            )
            .map_err(|error| error.to_string())?;
    }
    transaction.commit().map_err(|error| error.to_string())?;
    snapshot_for_user(&connection, &manager)
}

#[tauri::command]
pub(crate) fn add_product_stock(
    token: String,
    product_id: String,
    quantity: f64,
    state: tauri::State<DatabaseState>,
) -> Result<AppSnapshot, String> {
    let manager = require_manager(&state, &token)?;
    if !quantity.is_finite() || quantity <= 0.0 {
        return Err("入库数量必须大于0".to_string());
    }
    let mut connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    transaction
        .query_row(
            "SELECT 1 FROM products WHERE id=?1",
            params![product_id],
            |_| Ok(()),
        )
        .map_err(|_| "产品不存在".to_string())?;
    let now = Utc::now().to_rfc3339();
    transaction
        .execute(
            "INSERT INTO product_stock_entries(id,product_id,quantity,created_at,note)
             VALUES (?1,?2,?3,?4,'手动增加库存')",
            params![Uuid::new_v4().to_string(), product_id, quantity, now],
        )
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "UPDATE products SET updated_at=?1 WHERE id=?2",
            params![now, product_id],
        )
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())?;
    snapshot_for_user(&connection, &manager)
}

#[tauri::command]
pub(crate) fn create_supply_purchase(
    token: String,
    input: SupplyPurchaseInput,
    state: tauri::State<DatabaseState>,
) -> Result<AppSnapshot, String> {
    let manager = require_manager(&state, &token)?;
    if input.name.trim().is_empty()
        || input.purchased_at.trim().is_empty()
        || !input.amount.is_finite()
        || input.amount < 0.0
    {
        return Err("物料名称、采购日期或采购价格无效".to_string());
    }
    let connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    connection
        .execute(
            "INSERT INTO supply_purchases
             (id,name,category,quantity,unit,amount,purchased_at,note,created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                Uuid::new_v4().to_string(),
                input.name.trim(),
                "其他",
                1.0,
                "次",
                round_money(input.amount),
                input.purchased_at.trim(),
                "",
                Utc::now().to_rfc3339()
            ],
        )
        .map_err(|error| error.to_string())?;
    snapshot_for_user(&connection, &manager)
}

#[tauri::command]
pub(crate) fn delete_supply_purchase(
    token: String,
    purchase_id: String,
    state: tauri::State<DatabaseState>,
) -> Result<AppSnapshot, String> {
    let manager = require_manager(&state, &token)?;
    let connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    let changed = connection
        .execute(
            "DELETE FROM supply_purchases WHERE id=?1",
            params![purchase_id],
        )
        .map_err(|error| error.to_string())?;
    if changed == 0 {
        return Err("采购记录不存在".to_string());
    }
    snapshot_for_user(&connection, &manager)
}
