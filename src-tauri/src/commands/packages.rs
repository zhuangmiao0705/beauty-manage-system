use crate::{
    commands::auth::{require_manager, require_session},
    database::{
        employee_compensation_for_name, round_money, snapshot_for_user, validate_active_employee,
    },
    models::{
        AppSnapshot, EmployeeCompensation, PackageConsumptionInput, PackageDefinitionInput,
        PackagePurchaseInput,
    },
    state::DatabaseState,
};
use chrono::{DateTime, Duration, Utc};
use rusqlite::params;
use uuid::Uuid;

fn validate_package(input: &PackageDefinitionInput) -> Result<(), String> {
    if input.name.trim().is_empty()
        || !input.price.is_finite()
        || input.price < 0.0
        || (input.limit_type != "count" && input.limit_type != "time")
        || (input.limit_type == "count" && input.total_uses <= 0)
        || (input.limit_type == "time" && input.validity_days <= 0)
        || (input.package_type != "套盒" && input.package_type != "普通")
    {
        return Err("套餐名称、价格、限制方式或类型无效".to_string());
    }
    Ok(())
}

fn package_service_details(
    package_type: &str,
    compensation: &EmployeeCompensation,
) -> (&'static str, f64) {
    if package_type == "普通" {
        (
            "普通手工",
            round_money(compensation.normal_service_commission),
        )
    } else {
        (
            "套盒手工",
            round_money(compensation.package_service_commission),
        )
    }
}

fn time_package_is_active(expires_at: Option<&str>, now: DateTime<Utc>) -> bool {
    expires_at
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .is_some_and(|value| value.with_timezone(&Utc) > now)
}

fn resolve_payment_amounts(
    payment_method: &str,
    principal_balance: f64,
    price: f64,
    submitted_balance_amount: f64,
    submitted_cash_amount: f64,
) -> Result<(f64, f64), String> {
    let price = round_money(price);
    if price == 0.0 {
        return match payment_method {
            "会员余额" | "现金" | "余额现金组合支付" => Ok((0.0, 0.0)),
            _ => Err("套餐支付方式无效".to_string()),
        };
    }
    let (balance_amount, cash_amount) = match payment_method {
        "会员余额" => (price, 0.0),
        "现金" => (0.0, price),
        "余额现金组合支付" => {
            if !submitted_balance_amount.is_finite() || !submitted_cash_amount.is_finite() {
                return Err("支付金额无效".to_string());
            }
            let balance_amount = round_money(submitted_balance_amount);
            let cash_amount = round_money(submitted_cash_amount);
            if balance_amount <= 0.0 || cash_amount <= 0.0 {
                return Err("组合支付的余额支付和现金支付金额都必须大于0".to_string());
            }
            let total = round_money(balance_amount + cash_amount);
            if total < price {
                return Err(format!(
                    "支付总金额不足套餐价格，还差{:.2}元",
                    price - total
                ));
            }
            if total > price {
                return Err("支付总金额不能超过套餐价格".to_string());
            }
            (balance_amount, cash_amount)
        }
        _ => return Err("套餐支付方式无效".to_string()),
    };
    if principal_balance + 0.001 < balance_amount {
        return Err("实付本金余额不足，赠送余额不可购买套餐".to_string());
    }
    Ok((balance_amount, cash_amount))
}

#[tauri::command]
pub(crate) fn create_package(
    token: String,
    input: PackageDefinitionInput,
    state: tauri::State<DatabaseState>,
) -> Result<AppSnapshot, String> {
    let manager = require_manager(&state, &token)?;
    validate_package(&input)?;
    let connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    let now = Utc::now().to_rfc3339();
    let total_uses = if input.limit_type == "count" {
        input.total_uses
    } else {
        1
    };
    let validity_days = if input.limit_type == "time" {
        input.validity_days
    } else {
        0
    };
    connection
        .execute(
            "INSERT INTO packages
             (id,name,price,total_uses,limit_type,validity_days,package_type,status,created_at,updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,'active',?8,?8)",
            params![
                Uuid::new_v4().to_string(),
                input.name.trim(),
                round_money(input.price),
                total_uses,
                input.limit_type,
                validity_days,
                input.package_type,
                now
            ],
        )
        .map_err(|error| {
            if error.to_string().contains("UNIQUE") {
                "套餐名称已存在".to_string()
            } else {
                error.to_string()
            }
        })?;
    snapshot_for_user(&connection, &manager)
}

#[tauri::command]
pub(crate) fn update_package(
    token: String,
    package_id: String,
    input: PackageDefinitionInput,
    state: tauri::State<DatabaseState>,
) -> Result<AppSnapshot, String> {
    let manager = require_manager(&state, &token)?;
    validate_package(&input)?;
    let connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    let total_uses = if input.limit_type == "count" {
        input.total_uses
    } else {
        1
    };
    let validity_days = if input.limit_type == "time" {
        input.validity_days
    } else {
        0
    };
    let changed = connection
        .execute(
            "UPDATE packages SET name=?1,price=?2,total_uses=?3,limit_type=?4,
             validity_days=?5,package_type=?6,updated_at=?7 WHERE id=?8",
            params![
                input.name.trim(),
                round_money(input.price),
                total_uses,
                input.limit_type,
                validity_days,
                input.package_type,
                Utc::now().to_rfc3339(),
                package_id
            ],
        )
        .map_err(|error| {
            if error.to_string().contains("UNIQUE") {
                "套餐名称已存在".to_string()
            } else {
                error.to_string()
            }
        })?;
    if changed == 0 {
        return Err("套餐不存在".to_string());
    }
    snapshot_for_user(&connection, &manager)
}

#[tauri::command]
pub(crate) fn set_package_status(
    token: String,
    package_id: String,
    status: String,
    state: tauri::State<DatabaseState>,
) -> Result<AppSnapshot, String> {
    let manager = require_manager(&state, &token)?;
    if status != "active" && status != "inactive" {
        return Err("套餐状态无效".to_string());
    }
    let connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    let changed = connection
        .execute(
            "UPDATE packages SET status=?1,updated_at=?2 WHERE id=?3",
            params![status, Utc::now().to_rfc3339(), package_id],
        )
        .map_err(|error| error.to_string())?;
    if changed == 0 {
        return Err("套餐不存在".to_string());
    }
    snapshot_for_user(&connection, &manager)
}

#[tauri::command]
pub(crate) fn purchase_package(
    token: String,
    input: PackagePurchaseInput,
    state: tauri::State<DatabaseState>,
) -> Result<AppSnapshot, String> {
    let operator = require_session(&state, &token)?;
    let mut connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    let employee = validate_active_employee(&connection, &input.employee)?;
    let compensation = employee_compensation_for_name(&connection, &employee)?;
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    let (member_name, principal_balance, gift_balance): (String, f64, f64) = transaction
        .query_row(
            "SELECT name,principal_balance,gift_balance FROM members
             WHERE id=?1 AND status='active'",
            params![input.member_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|_| "没有找到启用状态的会员".to_string())?;
    let (package_name, package_type, price, total_uses, limit_type, validity_days): (
        String,
        String,
        f64,
        i64,
        String,
        i64,
    ) = transaction
        .query_row(
            "SELECT name,package_type,price,total_uses,limit_type,validity_days FROM packages
             WHERE id=?1 AND status='active'",
            params![input.package_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .map_err(|_| "没有找到启用状态的套餐".to_string())?;
    let (balance_payment_amount, cash_payment_amount) = resolve_payment_amounts(
        &input.payment_method,
        principal_balance,
        price,
        input.balance_payment_amount,
        input.cash_payment_amount,
    )?;
    let principal_balance_after = round_money(principal_balance - balance_payment_amount);
    let balance_after = round_money(principal_balance_after + gift_balance);
    let now_value = Utc::now();
    let now = now_value.to_rfc3339();
    let expires_at = if limit_type == "time" {
        Some((now_value + Duration::days(validity_days)).to_rfc3339())
    } else {
        None
    };
    let purchase_id = Uuid::new_v4().to_string();
    let transaction_id = if price > 0.0 {
        Uuid::new_v4().to_string()
    } else {
        String::new()
    };
    transaction
        .execute(
            "UPDATE members SET balance=?1,principal_balance=?2,
             total_consumption=total_consumption+?3,last_visit=?4 WHERE id=?5",
            params![
                balance_after,
                principal_balance_after,
                price,
                now,
                input.member_id
            ],
        )
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "INSERT INTO package_purchases
             (id,member_id,member_name,employee,package_id,package_name,package_type,price,total_uses,
              remaining_uses,limit_type,validity_days,expires_at,payment_method,
              balance_payment_amount,cash_payment_amount,status,purchased_at,last_consumed_at,
              commission,commission_rule_version,transaction_id)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?9,?10,?11,?12,?13,?14,?15,
                     'active',?16,NULL,?17,3,?18)",
            params![
                purchase_id,
                input.member_id,
                member_name,
                employee,
                input.package_id,
                package_name,
                package_type,
                price,
                total_uses,
                limit_type,
                validity_days,
                expires_at,
                input.payment_method,
                balance_payment_amount,
                cash_payment_amount,
                now,
                round_money(cash_payment_amount * compensation.base_commission_rate),
                transaction_id
            ],
        )
        .map_err(|error| error.to_string())?;
    if price > 0.0 {
        transaction
            .execute(
                "INSERT INTO transactions
                 (id,member_id,member_name,type,amount,gift_amount,balance_after,payment_method,item,
                  employee,commission,commission_rule_version,created_at,note,status,source_type,source_id)
                 VALUES (?1,?2,?3,'consume',?4,0,?5,?6,?7,?8,0,2,?9,?10,'active',
                         'package_purchase',?11)",
                params![
                    transaction_id,
                    input.member_id,
                    member_name,
                    price,
                    balance_after,
                    input.payment_method,
                    format!("套餐购买：{package_name}"),
                    employee,
                    now,
                    format!("套餐购买记录：{purchase_id}"),
                    purchase_id
                ],
            )
            .map_err(|error| error.to_string())?;
    }
    transaction.commit().map_err(|error| error.to_string())?;
    snapshot_for_user(&connection, &operator)
}

#[cfg(test)]
mod tests {
    use super::{package_service_details, resolve_payment_amounts, time_package_is_active};
    use crate::models::EmployeeCompensation;
    use chrono::{Duration, Utc};

    fn compensation() -> EmployeeCompensation {
        EmployeeCompensation {
            employee_id: "e1".to_string(),
            base_salary: 1800.0,
            base_commission_rate: 0.1,
            performance_target: 10000.0,
            excess_commission_rate: 0.02,
            meal_allowance_per_day: 10.0,
            attendance_bonus: 300.0,
            normal_service_commission: 5.0,
            package_service_commission: 10.0,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn package_payments_are_resolved_and_validated() {
        assert!(resolve_payment_amounts("会员余额", 70.0, 80.0, 0.0, 0.0).is_err());
        assert_eq!(
            resolve_payment_amounts("会员余额", 80.0, 80.0, 0.0, 0.0).unwrap(),
            (80.0, 0.0)
        );
        assert_eq!(
            resolve_payment_amounts("现金", 0.0, 80.0, 0.0, 0.0).unwrap(),
            (0.0, 80.0)
        );
        assert_eq!(
            resolve_payment_amounts("会员余额", 0.0, 0.0, 0.0, 0.0).unwrap(),
            (0.0, 0.0)
        );
        assert_eq!(
            resolve_payment_amounts("余额现金组合支付", 50.0, 80.0, 30.0, 50.0).unwrap(),
            (30.0, 50.0)
        );
        assert!(
            resolve_payment_amounts("余额现金组合支付", 50.0, 80.0, 30.0, 40.0)
                .unwrap_err()
                .contains("不足")
        );
        assert!(
            resolve_payment_amounts("余额现金组合支付", 20.0, 80.0, 30.0, 50.0)
                .unwrap_err()
                .contains("本金余额不足")
        );
    }

    #[test]
    fn package_type_selects_the_matching_service_commission() {
        assert_eq!(
            package_service_details("普通", &compensation()),
            ("普通手工", 5.0)
        );
        assert_eq!(
            package_service_details("套盒", &compensation()),
            ("套盒手工", 10.0)
        );
    }

    #[test]
    fn time_package_is_only_active_before_its_expiration() {
        let now = Utc::now();
        let future = (now + Duration::days(1)).to_rfc3339();
        let past = (now - Duration::days(1)).to_rfc3339();
        assert!(time_package_is_active(Some(&future), now));
        assert!(!time_package_is_active(Some(&past), now));
        assert!(!time_package_is_active(None, now));
    }
}

#[tauri::command]
pub(crate) fn consume_package(
    token: String,
    input: PackageConsumptionInput,
    state: tauri::State<DatabaseState>,
) -> Result<AppSnapshot, String> {
    let operator = require_session(&state, &token)?;
    if input.duration <= 0 {
        return Err("服务时长必须大于0".to_string());
    }
    let mut connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    let employee = validate_active_employee(&connection, &input.employee)?;
    let compensation = employee_compensation_for_name(&connection, &employee)?;
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    let (
        member_id,
        member_name,
        package_name,
        package_type,
        remaining_uses,
        limit_type,
        expires_at,
    ): (String, String, String, String, i64, String, Option<String>) = transaction
        .query_row(
            "SELECT member_id,member_name,package_name,package_type,remaining_uses,
                    limit_type,expires_at
                 FROM package_purchases
                 WHERE id=?1 AND status='active'",
            params![input.purchase_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .map_err(|_| "套餐不存在或已结束".to_string())?;
    let now_value = Utc::now();
    if (limit_type == "count" && remaining_uses <= 0)
        || (limit_type == "time" && !time_package_is_active(expires_at.as_deref(), now_value))
    {
        return Err("套餐已过期或没有剩余次数".to_string());
    }
    let remaining_after = if limit_type == "count" {
        remaining_uses - 1
    } else {
        remaining_uses
    };
    let now = now_value.to_rfc3339();
    let service_id = Uuid::new_v4().to_string();
    let changed = if limit_type == "count" {
        transaction.execute(
            "UPDATE package_purchases SET remaining_uses=?1,last_consumed_at=?2,
             status=CASE WHEN ?1=0 THEN 'completed' ELSE 'active' END
             WHERE id=?3 AND remaining_uses=?4 AND status='active'",
            params![remaining_after, now, input.purchase_id, remaining_uses],
        )
    } else {
        transaction.execute(
            "UPDATE package_purchases SET last_consumed_at=?1
             WHERE id=?2 AND status='active' AND expires_at>?1",
            params![now, input.purchase_id],
        )
    }
    .map_err(|error| error.to_string())?;
    if changed != 1 {
        return Err("套餐状态已变化，请刷新后重试".to_string());
    }
    let (service_type, commission) = package_service_details(&package_type, &compensation);
    transaction
        .execute(
            "INSERT INTO services
             (id,member_id,member_name,employee,service_name,service_type,duration,amount,
              commission,commission_rule_version,package_purchase_id,created_at,status)
             VALUES (?1,?2,?3,?4,?5,?6,?7,0,?8,2,?9,?10,'completed')",
            params![
                service_id,
                member_id,
                member_name,
                employee,
                package_name,
                service_type,
                input.duration,
                commission,
                input.purchase_id,
                now
            ],
        )
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "INSERT INTO package_consumptions
             (id,package_purchase_id,member_id,member_name,employee,package_name,consumed_at,
              remaining_after,commission,service_id,duration,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![
                Uuid::new_v4().to_string(),
                input.purchase_id,
                member_id,
                member_name,
                employee,
                package_name,
                now,
                remaining_after,
                commission,
                service_id,
                input.duration,
                input.note.trim()
            ],
        )
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "UPDATE members SET last_visit=?1 WHERE id=?2",
            params![now, member_id],
        )
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())?;
    snapshot_for_user(&connection, &operator)
}
