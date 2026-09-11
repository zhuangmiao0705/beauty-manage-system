use crate::{
    commands::auth::{require_manager, require_session, revoke_account_sessions},
    database::{
        employee_compensation_for_name, round_money, snapshot, snapshot_for_user,
        validate_active_employee,
    },
    models::{AppSnapshot, EmployeeInput, MemberInput, ServiceInput, TransactionInput},
    state::DatabaseState,
};
use chrono::{DateTime, Utc};
use rusqlite::{params, Transaction};
use uuid::Uuid;

fn valid_external_payment_method(value: &str) -> bool {
    matches!(value, "微信支付" | "支付宝" | "现金" | "银行卡")
}

#[derive(Debug, PartialEq)]
struct ServicePaymentBreakdown {
    balance_payment: f64,
    external_payment: f64,
    gift_deduction: f64,
    principal_deduction: f64,
}

fn resolve_service_payment(
    principal_balance: f64,
    gift_balance: f64,
    price: f64,
) -> ServicePaymentBreakdown {
    let balance_payment = round_money((principal_balance + gift_balance).min(price));
    let gift_deduction = round_money(gift_balance.min(balance_payment));
    ServicePaymentBreakdown {
        balance_payment,
        external_payment: round_money(price - balance_payment),
        gift_deduction,
        principal_deduction: round_money(balance_payment - gift_deduction),
    }
}

pub(crate) fn insert_normal_service(
    transaction: &Transaction<'_>,
    service_id: &str,
    member_id: Option<&str>,
    guest_name: &str,
    employee: &str,
    project_id: &str,
    external_payment_method: &str,
    allow_inactive_project: bool,
    now: &str,
) -> Result<String, String> {
    let (project_name, duration, price): (String, i64, f64) = transaction
        .query_row(
            "SELECT name,duration,price FROM projects
             WHERE id=?1 AND (status='active' OR ?2=1)",
            params![project_id, i64::from(allow_inactive_project)],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|_| "没有找到启用状态的服务项目".to_string())?;
    let compensation = employee_compensation_for_name(transaction, employee)?;
    let mut balance_payment_amount = 0.0;
    let mut external_payment_amount = round_money(price);
    let mut gift_deduction = 0.0;
    let mut principal_deduction = 0.0;
    let (resolved_member_id, customer_name, balance_after) = if let Some(member_id) = member_id {
        let (member_name, principal_balance, gift_balance): (String, f64, f64) = transaction
            .query_row(
                "SELECT name,principal_balance,gift_balance FROM members
                 WHERE id=?1 AND status='active'",
                params![member_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|_| "没有找到启用状态的会员".to_string())?;
        let payment = resolve_service_payment(principal_balance, gift_balance, price);
        balance_payment_amount = payment.balance_payment;
        external_payment_amount = payment.external_payment;
        gift_deduction = payment.gift_deduction;
        principal_deduction = payment.principal_deduction;
        let principal_after = round_money(principal_balance - principal_deduction);
        let gift_after = round_money(gift_balance - gift_deduction);
        let balance_after = round_money(principal_after + gift_after);
        transaction
            .execute(
                "UPDATE members SET balance=?1,principal_balance=?2,gift_balance=?3,
                 total_consumption=total_consumption+?4,last_visit=?5 WHERE id=?6",
                params![
                    balance_after,
                    principal_after,
                    gift_after,
                    price,
                    now,
                    member_id
                ],
            )
            .map_err(|error| error.to_string())?;
        (Some(member_id.to_string()), member_name, balance_after)
    } else {
        let guest_name = guest_name.trim();
        if guest_name.is_empty() {
            return Err("请填写游客称呼".to_string());
        }
        (None, guest_name.to_string(), 0.0)
    };
    if external_payment_amount > 0.0 && !valid_external_payment_method(external_payment_method) {
        return Err("请选择实际支付方式".to_string());
    }
    let payment_method = if balance_payment_amount > 0.0 && external_payment_amount > 0.0 {
        format!("会员余额+{external_payment_method}")
    } else if balance_payment_amount > 0.0 {
        "会员余额".to_string()
    } else {
        external_payment_method.to_string()
    };
    let transaction_id = Uuid::new_v4().to_string();
    transaction
        .execute(
            "INSERT INTO services
             (id,member_id,member_name,employee,service_name,service_type,duration,amount,
              commission,commission_rule_version,package_purchase_id,project_id,
              balance_payment_amount,external_payment_amount,payment_method,gift_deduction,
              principal_deduction,transaction_id,created_at,status)
             VALUES (?1,?2,?3,?4,?5,'普通手工',?6,?7,?8,2,NULL,?9,?10,?11,?12,
                     ?13,?14,?15,?16,'completed')",
            params![
                service_id,
                resolved_member_id,
                customer_name,
                employee,
                project_name,
                duration,
                round_money(price),
                round_money(compensation.normal_service_commission),
                project_id,
                balance_payment_amount,
                external_payment_amount,
                payment_method,
                gift_deduction,
                principal_deduction,
                transaction_id,
                now
            ],
        )
        .map_err(|error| {
            if error.to_string().contains("UNIQUE") {
                "该服务已经登记，请勿重复提交".to_string()
            } else {
                error.to_string()
            }
        })?;
    transaction
        .execute(
            "INSERT INTO transactions
             (id,member_id,member_name,type,amount,gift_amount,balance_after,payment_method,item,
              employee,commission,commission_rule_version,created_at,note,status,source_type,source_id)
             VALUES (?1,?2,?3,'consume',?4,0,?5,?6,?7,?8,0,2,?9,'普通消费自动结算',
                     'active','service',?10)",
            params![
                transaction_id,
                resolved_member_id,
                customer_name,
                round_money(price),
                balance_after,
                payment_method,
                project_name,
                employee,
                now,
                service_id
            ],
        )
        .map_err(|error| error.to_string())?;
    Ok(service_id.to_string())
}

#[tauri::command]
pub(crate) fn get_snapshot(
    token: String,
    state: tauri::State<DatabaseState>,
) -> Result<AppSnapshot, String> {
    let user = require_session(&state, &token)?;
    let connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    snapshot_for_user(&connection, &user)
}

#[tauri::command]
pub(crate) fn create_member(
    token: String,
    input: MemberInput,
    state: tauri::State<DatabaseState>,
) -> Result<AppSnapshot, String> {
    let operator = require_session(&state, &token)?;
    if input.name.trim().is_empty()
        || input.phone.trim().is_empty()
        || input.initial_balance < 0.0
        || input.gift_amount < 0.0
        || (input.gift_amount > 0.0 && input.initial_balance <= 0.0)
    {
        return Err("会员信息、充值金额或赠送金额无效".to_string());
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
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let balance = round_money(input.initial_balance + input.gift_amount);
    transaction
        .execute(
            "INSERT INTO members
             (id,name,phone,balance,principal_balance,gift_balance,total_recharge,
              total_consumption,join_date,last_visit,status)
             VALUES (?1,?2,?3,?4,?5,?6,?5,0,?7,?7,'active')",
            params![
                id,
                input.name.trim(),
                input.phone.trim(),
                balance,
                input.initial_balance,
                input.gift_amount,
                now
            ],
        )
        .map_err(|error| error.to_string())?;
    if input.initial_balance > 0.0 {
        transaction
            .execute(
                "INSERT INTO transactions
                 (id,member_id,member_name,type,amount,gift_amount,balance_after,payment_method,
                  item,employee,commission,commission_rule_version,created_at,note)
                 VALUES (?1,?2,?3,'recharge',?4,?5,?6,'现金','开卡充值',?7,?8,2,?9,'新会员开卡')",
                params![
                    Uuid::new_v4().to_string(),
                    id,
                    input.name.trim(),
                    input.initial_balance,
                    input.gift_amount,
                    balance,
                    employee,
                    round_money(input.initial_balance * compensation.base_commission_rate),
                    now
                ],
            )
            .map_err(|error| error.to_string())?;
    }
    transaction.commit().map_err(|error| error.to_string())?;
    snapshot_for_user(&connection, &operator)
}

#[tauri::command]
pub(crate) fn create_transaction(
    token: String,
    input: TransactionInput,
    state: tauri::State<DatabaseState>,
) -> Result<AppSnapshot, String> {
    let operator = require_session(&state, &token)?;
    if input.amount <= 0.0
        || input.gift_amount < 0.0
        || input.r#type != "recharge"
        || input.payment_method == "会员余额"
    {
        return Err("会员管理仅支持充值，消费请通过登记消费完成".to_string());
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
    let (member_name, principal_balance, gift_balance): (String, f64, f64) = transaction
        .query_row(
            "SELECT name,principal_balance,gift_balance FROM members
             WHERE id=?1 AND status='active'",
            params![input.member_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|_| "没有找到启用状态的会员".to_string())?;
    let new_principal_balance = round_money(principal_balance + input.amount);
    let new_gift_balance = round_money(gift_balance + input.gift_amount);
    let new_balance = round_money(new_principal_balance + new_gift_balance);
    let commission = round_money(input.amount * compensation.base_commission_rate);
    let now = Utc::now().to_rfc3339();
    transaction
        .execute(
            "UPDATE members SET balance=?1,principal_balance=?2,gift_balance=?3,
             total_recharge=total_recharge+?4,last_visit=?5 WHERE id=?6",
            params![
                new_balance,
                new_principal_balance,
                new_gift_balance,
                input.amount,
                now,
                input.member_id
            ],
        )
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "INSERT INTO transactions
             (id,member_id,member_name,type,amount,gift_amount,balance_after,payment_method,item,
              employee,commission,commission_rule_version,created_at,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,2,?12,?13)",
            params![
                Uuid::new_v4().to_string(),
                input.member_id,
                member_name,
                input.r#type,
                input.amount,
                input.gift_amount,
                new_balance,
                input.payment_method,
                input.item,
                employee,
                commission,
                now,
                input.note
            ],
        )
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())?;
    snapshot_for_user(&connection, &operator)
}

#[tauri::command]
pub(crate) fn create_service(
    token: String,
    input: ServiceInput,
    state: tauri::State<DatabaseState>,
) -> Result<AppSnapshot, String> {
    let operator = require_session(&state, &token)?;
    if input.request_id.trim().is_empty() || input.project_id.trim().is_empty() {
        return Err("请选择服务项目".to_string());
    }
    let mut connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    let employee = validate_active_employee(&connection, &input.employee)?;
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    let now = Utc::now().to_rfc3339();
    insert_normal_service(
        &transaction,
        input.request_id.trim(),
        (!input.member_id.trim().is_empty()).then_some(input.member_id.as_str()),
        &input.guest_name,
        &employee,
        &input.project_id,
        &input.external_payment_method,
        false,
        &now,
    )?;
    transaction.commit().map_err(|error| error.to_string())?;
    snapshot_for_user(&connection, &operator)
}

#[tauri::command]
pub(crate) fn cancel_service(
    token: String,
    service_id: String,
    state: tauri::State<DatabaseState>,
) -> Result<AppSnapshot, String> {
    let manager = require_manager(&state, &token)?;
    let mut connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    let (
        member_id,
        amount,
        gift_deduction,
        principal_deduction,
        transaction_id,
        service_type,
        package_purchase_id,
    ): (
        Option<String>,
        f64,
        f64,
        f64,
        Option<String>,
        String,
        Option<String>,
    ) = transaction
        .query_row(
            "SELECT member_id,amount,gift_deduction,principal_deduction,transaction_id,
                    service_type,package_purchase_id
             FROM services WHERE id=?1 AND status='completed'",
            params![service_id],
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
        .map_err(|_| "该服务不存在、已撤销或不支持撤销".to_string())?;
    let now = Utc::now().to_rfc3339();
    if let Some(package_purchase_id) = package_purchase_id {
        let consumption_id: String = transaction
            .query_row(
                "SELECT id FROM package_consumptions
                 WHERE service_id=?1 AND package_purchase_id=?2 AND status='active'",
                params![service_id, package_purchase_id],
                |row| row.get(0),
            )
            .map_err(|_| "对应的套餐消耗流水不存在或已撤销".to_string())?;
        let (limit_type, remaining_uses, total_uses, expires_at): (
            String,
            i64,
            i64,
            Option<String>,
        ) = transaction
            .query_row(
                "SELECT limit_type,remaining_uses,total_uses,expires_at
                 FROM package_purchases WHERE id=?1",
                params![package_purchase_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .map_err(|_| "套餐购买记录不存在".to_string())?;
        let changed = transaction
            .execute(
                "UPDATE package_consumptions SET status='cancelled',cancelled_at=?1
                 WHERE id=?2 AND status='active'",
                params![now, consumption_id],
            )
            .map_err(|error| error.to_string())?;
        if changed != 1 {
            return Err("该套餐消耗已撤销，请刷新后重试".to_string());
        }
        let last_consumed_at: Option<String> = transaction
            .query_row(
                "SELECT MAX(consumed_at) FROM package_consumptions
                 WHERE package_purchase_id=?1 AND status='active'",
                params![package_purchase_id],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        let (restored_uses, package_status) = if limit_type == "count" {
            ((remaining_uses + 1).min(total_uses), "active")
        } else {
            let is_active = expires_at
                .as_deref()
                .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                .is_some_and(|value| value.with_timezone(&Utc) > Utc::now());
            (
                remaining_uses,
                if is_active { "active" } else { "completed" },
            )
        };
        transaction
            .execute(
                "UPDATE package_purchases
                 SET remaining_uses=?1,last_consumed_at=?2,status=?3 WHERE id=?4",
                params![
                    restored_uses,
                    last_consumed_at,
                    package_status,
                    package_purchase_id
                ],
            )
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "UPDATE appointments SET status='in_service',completed_service_id=NULL,updated_at=?1
                 WHERE completed_service_id=?2 AND status='completed'",
                params![now, service_id],
            )
            .map_err(|error| error.to_string())?;
    } else {
        let transaction_id = transaction_id.filter(|value| !value.is_empty());
        if service_type != "普通手工" || transaction_id.is_none() {
            return Err("该服务不支持撤销".to_string());
        }
        if let Some(member_id) = member_id {
            let (principal_balance, gift_balance, total_consumption): (f64, f64, f64) =
                transaction
                    .query_row(
                        "SELECT principal_balance,gift_balance,total_consumption FROM members WHERE id=?1",
                        params![member_id],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                    )
                    .map_err(|_| "会员不存在，无法撤销".to_string())?;
            let principal_after = round_money(principal_balance + principal_deduction);
            let gift_after = round_money(gift_balance + gift_deduction);
            transaction
                .execute(
                    "UPDATE members SET principal_balance=?1,gift_balance=?2,balance=?3,
                     total_consumption=?4 WHERE id=?5",
                    params![
                        principal_after,
                        gift_after,
                        round_money(principal_after + gift_after),
                        round_money((total_consumption - amount).max(0.0)),
                        member_id
                    ],
                )
                .map_err(|error| error.to_string())?;
        }
        transaction
            .execute(
                "UPDATE transactions SET status='cancelled',note=note||'；服务已撤销'
                 WHERE id=?1 AND status='active'",
                params![transaction_id],
            )
            .map_err(|error| error.to_string())?;
    }
    transaction
        .execute(
            "UPDATE services SET status='cancelled',cancelled_at=?1 WHERE id=?2",
            params![now, service_id],
        )
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())?;
    snapshot_for_user(&connection, &manager)
}

#[tauri::command]
pub(crate) fn create_employee(
    token: String,
    input: EmployeeInput,
    state: tauri::State<DatabaseState>,
) -> Result<AppSnapshot, String> {
    require_manager(&state, &token)?;
    if input.name.trim().is_empty()
        || input.base_salary < 0.0
        || !(0.0..=1.0).contains(&input.base_commission_rate)
        || input.performance_target < 0.0
        || !(0.0..=1.0).contains(&input.excess_commission_rate)
        || input.meal_allowance_per_day < 0.0
        || input.attendance_bonus < 0.0
        || input.normal_service_commission < 0.0
        || input.package_service_commission < 0.0
    {
        return Err("员工信息无效".to_string());
    }
    let mut connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    transaction
        .execute(
            "INSERT INTO employees (id,name,role,commission_rate,status,color,created_at)
             VALUES (?1,?2,?3,0,'active',?4,?5)",
            params![id, input.name.trim(), input.role, input.color, now],
        )
        .map_err(|error| {
            if error.to_string().contains("UNIQUE") {
                "员工姓名已存在".to_string()
            } else {
                error.to_string()
            }
        })?;
    transaction
        .execute(
            "INSERT INTO employee_status_events(id,employee_id,employee_name,status,effective_at)
             VALUES (?1,?2,?3,'active',?4)",
            params![Uuid::new_v4().to_string(), id, input.name.trim(), now],
        )
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "INSERT INTO employee_compensations
             (employee_id,base_salary,base_commission_rate,performance_target,
              excess_commission_rate,meal_allowance_per_day,attendance_bonus,
              normal_service_commission,package_service_commission,created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                id,
                round_money(input.base_salary),
                input.base_commission_rate,
                round_money(input.performance_target),
                input.excess_commission_rate,
                round_money(input.meal_allowance_per_day),
                round_money(input.attendance_bonus),
                round_money(input.normal_service_commission),
                round_money(input.package_service_commission),
                now
            ],
        )
        .map_err(|error| error.to_string())?;
    transaction.commit().map_err(|error| error.to_string())?;
    snapshot(&connection)
}

#[tauri::command]
pub(crate) fn set_employee_status(
    token: String,
    employee_id: String,
    status: String,
    state: tauri::State<DatabaseState>,
) -> Result<AppSnapshot, String> {
    require_manager(&state, &token)?;
    if status != "active" && status != "inactive" {
        return Err("员工状态无效".to_string());
    }
    let mut connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    let (employee_name, current_status): (String, String) = connection
        .query_row(
            "SELECT name,status FROM employees WHERE id=?1",
            params![employee_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| "员工不存在".to_string())?;
    if current_status == status {
        return snapshot(&connection);
    }
    let linked_account_ids = if status == "inactive" {
        let mut statement = connection
            .prepare("SELECT id FROM accounts WHERE employee_id=?1")
            .map_err(|error| error.to_string())?;
        let account_ids = statement
            .query_map(params![employee_id], |row| row.get::<_, String>(0))
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        account_ids
    } else {
        Vec::new()
    };
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    let now = Utc::now().to_rfc3339();
    transaction
        .execute(
            "UPDATE employees SET status=?1 WHERE id=?2",
            params![status, employee_id],
        )
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "INSERT INTO employee_status_events(id,employee_id,employee_name,status,effective_at)
             VALUES (?1,?2,?3,?4,?5)",
            params![
                Uuid::new_v4().to_string(),
                employee_id,
                employee_name,
                status,
                now
            ],
        )
        .map_err(|error| error.to_string())?;
    if status == "inactive" {
        transaction
            .execute(
                "UPDATE accounts SET status='inactive',updated_at=?1 WHERE employee_id=?2",
                params![now, employee_id],
            )
            .map_err(|error| error.to_string())?;
    }
    transaction.commit().map_err(|error| error.to_string())?;
    let data = snapshot(&connection)?;
    drop(connection);
    for account_id in linked_account_ids {
        revoke_account_sessions(&state, &account_id)?;
    }
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::{resolve_service_payment, ServicePaymentBreakdown};

    #[test]
    fn service_payment_uses_gift_first_and_combines_when_balance_is_insufficient() {
        assert_eq!(
            resolve_service_payment(100.0, 50.0, 80.0),
            ServicePaymentBreakdown {
                balance_payment: 80.0,
                external_payment: 0.0,
                gift_deduction: 50.0,
                principal_deduction: 30.0,
            }
        );
        assert_eq!(
            resolve_service_payment(20.0, 10.0, 80.0),
            ServicePaymentBreakdown {
                balance_payment: 30.0,
                external_payment: 50.0,
                gift_deduction: 10.0,
                principal_deduction: 20.0,
            }
        );
    }
}
