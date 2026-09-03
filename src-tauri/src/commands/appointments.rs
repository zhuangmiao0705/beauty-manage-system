use crate::{
    commands::auth::require_session,
    commands::salon::insert_normal_service,
    database::{
        employee_compensation_for_name, round_money, snapshot_for_user, validate_active_employee,
    },
    models::{AppSnapshot, AppointmentCompletionInput, AppointmentInput},
    state::DatabaseState,
};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection, Transaction};
use uuid::Uuid;

struct ResolvedAppointment {
    customer_type: String,
    member_id: Option<String>,
    customer_name: String,
    customer_phone: String,
    employee: String,
    service_type: String,
    service_name: String,
    project_id: Option<String>,
    package_purchase_id: Option<String>,
    starts_at: String,
    starts_at_value: DateTime<Utc>,
    duration: i64,
    note: String,
}

fn parse_starts_at(value: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(value)
        .map(|date| date.with_timezone(&Utc))
        .map_err(|_| "预约时间格式无效".to_string())
}

fn time_ranges_overlap(
    starts_at: DateTime<Utc>,
    duration: i64,
    existing_starts_at: DateTime<Utc>,
    existing_duration: i64,
) -> bool {
    let ends_at = starts_at + Duration::minutes(duration);
    let existing_ends_at = existing_starts_at + Duration::minutes(existing_duration);
    starts_at < existing_ends_at && existing_starts_at < ends_at
}

fn ensure_no_conflict(
    connection: &Connection,
    employee: &str,
    starts_at: DateTime<Utc>,
    duration: i64,
    excluded_id: Option<&str>,
) -> Result<(), String> {
    let mut statement = connection
        .prepare(
            "SELECT id,customer_name,starts_at,duration FROM appointments
             WHERE employee=?1 AND status NOT IN ('cancelled','no_show')",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map(params![employee], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })
        .map_err(|error| error.to_string())?;
    for row in rows {
        let (id, customer_name, existing_starts_at, existing_duration) =
            row.map_err(|error| error.to_string())?;
        if excluded_id == Some(id.as_str()) {
            continue;
        }
        let Ok(existing_starts_at) = parse_starts_at(&existing_starts_at) else {
            continue;
        };
        if time_ranges_overlap(starts_at, duration, existing_starts_at, existing_duration) {
            return Err(format!(
                "该员工在此时间段已有“{customer_name}”的预约，请调整预约时间或服务员工"
            ));
        }
    }
    Ok(())
}

fn resolve_appointment(
    connection: &Connection,
    input: &AppointmentInput,
) -> Result<ResolvedAppointment, String> {
    if input.duration <= 0 || input.duration > 1440 {
        return Err("预约时长必须在1至1440分钟之间".to_string());
    }
    let starts_at_value = parse_starts_at(&input.starts_at)?;
    let employee = validate_active_employee(connection, &input.employee)?;
    let (member_id, customer_name, customer_phone) = match input.customer_type.as_str() {
        "member" => {
            let (name, phone): (String, String) = connection
                .query_row(
                    "SELECT name,phone FROM members WHERE id=?1 AND status='active'",
                    params![input.member_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .map_err(|_| "没有找到启用状态的会员".to_string())?;
            (Some(input.member_id.clone()), name, phone)
        }
        "guest" => {
            if input.guest_name.trim().is_empty() || input.guest_phone.trim().is_empty() {
                return Err("请填写游客姓名和手机号".to_string());
            }
            (
                None,
                input.guest_name.trim().to_string(),
                input.guest_phone.trim().to_string(),
            )
        }
        _ => return Err("预约顾客类型无效".to_string()),
    };
    let (service_name, project_id, package_purchase_id, duration) =
        match input.service_type.as_str() {
            "普通手工" => {
                let (name, duration): (String, i64) = connection
                    .query_row(
                        "SELECT name,duration FROM projects WHERE id=?1 AND status='active'",
                        params![input.project_id],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .map_err(|_| "没有找到启用状态的服务项目".to_string())?;
                (name, Some(input.project_id.clone()), None, duration)
            }
            "套盒手工" => {
                let member_id = member_id
                    .as_deref()
                    .ok_or_else(|| "套盒消费只能选择会员".to_string())?;
                let package_name = connection
                    .query_row(
                        "SELECT package_name FROM package_purchases
                     WHERE id=?1 AND member_id=?2 AND package_type='套盒'
                       AND status='active' AND remaining_uses>0",
                        params![input.package_purchase_id, member_id],
                        |row| row.get::<_, String>(0),
                    )
                    .map_err(|_| "没有找到该会员的可用套盒".to_string())?;
                (
                    package_name,
                    None,
                    Some(input.package_purchase_id.clone()),
                    input.duration,
                )
            }
            _ => return Err("预约服务类型无效".to_string()),
        };
    Ok(ResolvedAppointment {
        customer_type: input.customer_type.clone(),
        member_id,
        customer_name,
        customer_phone,
        employee,
        service_type: input.service_type.clone(),
        service_name,
        project_id,
        package_purchase_id,
        starts_at: starts_at_value.to_rfc3339(),
        starts_at_value,
        duration,
        note: input.note.trim().to_string(),
    })
}

#[tauri::command]
pub(crate) fn create_appointment(
    token: String,
    input: AppointmentInput,
    state: tauri::State<DatabaseState>,
) -> Result<AppSnapshot, String> {
    let operator = require_session(&state, &token)?;
    let connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    let appointment = resolve_appointment(&connection, &input)?;
    ensure_no_conflict(
        &connection,
        &appointment.employee,
        appointment.starts_at_value,
        appointment.duration,
        None,
    )?;
    let now = Utc::now().to_rfc3339();
    connection
        .execute(
            "INSERT INTO appointments
             (id,customer_type,member_id,customer_name,customer_phone,employee,service_type,
              service_name,project_id,package_purchase_id,starts_at,duration,status,note,created_by,
              created_at,updated_at,completed_service_id)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,'pending',?13,?14,?15,?15,NULL)",
            params![
                Uuid::new_v4().to_string(),
                appointment.customer_type,
                appointment.member_id,
                appointment.customer_name,
                appointment.customer_phone,
                appointment.employee,
                appointment.service_type,
                appointment.service_name,
                appointment.project_id,
                appointment.package_purchase_id,
                appointment.starts_at,
                appointment.duration,
                appointment.note,
                operator.display_name,
                now
            ],
        )
        .map_err(|error| error.to_string())?;
    snapshot_for_user(&connection, &operator)
}

#[tauri::command]
pub(crate) fn update_appointment(
    token: String,
    appointment_id: String,
    input: AppointmentInput,
    state: tauri::State<DatabaseState>,
) -> Result<AppSnapshot, String> {
    let operator = require_session(&state, &token)?;
    let connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    let status = connection
        .query_row(
            "SELECT status FROM appointments WHERE id=?1",
            params![appointment_id],
            |row| row.get::<_, String>(0),
        )
        .map_err(|_| "预约记录不存在".to_string())?;
    if status != "pending" && status != "arrived" {
        return Err("当前预约状态不允许编辑".to_string());
    }
    let appointment = resolve_appointment(&connection, &input)?;
    ensure_no_conflict(
        &connection,
        &appointment.employee,
        appointment.starts_at_value,
        appointment.duration,
        Some(&appointment_id),
    )?;
    connection
        .execute(
            "UPDATE appointments SET customer_type=?1,member_id=?2,customer_name=?3,
             customer_phone=?4,employee=?5,service_type=?6,service_name=?7,
             project_id=?8,package_purchase_id=?9,starts_at=?10,duration=?11,note=?12,updated_at=?13
             WHERE id=?14",
            params![
                appointment.customer_type,
                appointment.member_id,
                appointment.customer_name,
                appointment.customer_phone,
                appointment.employee,
                appointment.service_type,
                appointment.service_name,
                appointment.project_id,
                appointment.package_purchase_id,
                appointment.starts_at,
                appointment.duration,
                appointment.note,
                Utc::now().to_rfc3339(),
                appointment_id
            ],
        )
        .map_err(|error| error.to_string())?;
    snapshot_for_user(&connection, &operator)
}

fn transition_allowed(current: &str, next: &str) -> bool {
    matches!(
        (current, next),
        ("pending", "arrived")
            | ("pending", "cancelled")
            | ("pending", "no_show")
            | ("arrived", "in_service")
            | ("arrived", "cancelled")
    )
}

#[tauri::command]
pub(crate) fn set_appointment_status(
    token: String,
    appointment_id: String,
    status: String,
    state: tauri::State<DatabaseState>,
) -> Result<AppSnapshot, String> {
    let operator = require_session(&state, &token)?;
    let connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    let current = connection
        .query_row(
            "SELECT status FROM appointments WHERE id=?1",
            params![appointment_id],
            |row| row.get::<_, String>(0),
        )
        .map_err(|_| "预约记录不存在".to_string())?;
    if !transition_allowed(&current, &status) {
        return Err("预约状态流转无效，请刷新后重试".to_string());
    }
    connection
        .execute(
            "UPDATE appointments SET status=?1,updated_at=?2 WHERE id=?3 AND status=?4",
            params![status, Utc::now().to_rfc3339(), appointment_id, current],
        )
        .map_err(|error| error.to_string())?;
    snapshot_for_user(&connection, &operator)
}

#[tauri::command]
pub(crate) fn complete_appointment(
    token: String,
    input: AppointmentCompletionInput,
    state: tauri::State<DatabaseState>,
) -> Result<AppSnapshot, String> {
    let operator = require_session(&state, &token)?;
    if input.duration <= 0 {
        return Err("实际服务时长必须大于0".to_string());
    }
    let mut connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    let appointment: (
        String,
        Option<String>,
        String,
        String,
        String,
        Option<String>,
        String,
        Option<String>,
    ) = connection
        .query_row(
            "SELECT customer_type,member_id,customer_name,employee,service_type,
                    package_purchase_id,service_name,project_id
             FROM appointments WHERE id=?1 AND status='in_service'",
            params![input.appointment_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                ))
            },
        )
        .map_err(|_| "预约不存在或尚未进入服务中状态".to_string())?;
    let employee = validate_active_employee(&connection, &appointment.3)?;
    let compensation = employee_compensation_for_name(&connection, &employee)?;
    let transaction = connection
        .transaction()
        .map_err(|error| error.to_string())?;
    let now = Utc::now().to_rfc3339();
    let service_id = Uuid::new_v4().to_string();
    if appointment.4 == "套盒手工" {
        complete_package_service(
            &transaction,
            &appointment,
            &employee,
            &input,
            &service_id,
            &now,
            round_money(compensation.package_service_commission),
        )?;
    } else {
        let project_id = appointment
            .7
            .as_deref()
            .ok_or_else(|| "预约缺少服务项目信息".to_string())?;
        insert_normal_service(
            &transaction,
            &service_id,
            appointment.1.as_deref(),
            &appointment.2,
            &employee,
            project_id,
            &input.external_payment_method,
            true,
            &now,
        )?;
    }
    let changed = transaction
        .execute(
            "UPDATE appointments SET status='completed',completed_service_id=?1,updated_at=?2
             WHERE id=?3 AND status='in_service'",
            params![service_id, now, input.appointment_id],
        )
        .map_err(|error| error.to_string())?;
    if changed != 1 {
        return Err("预约状态已变化，请刷新后重试".to_string());
    }
    transaction.commit().map_err(|error| error.to_string())?;
    snapshot_for_user(&connection, &operator)
}

fn complete_package_service(
    transaction: &Transaction<'_>,
    appointment: &(
        String,
        Option<String>,
        String,
        String,
        String,
        Option<String>,
        String,
        Option<String>,
    ),
    employee: &str,
    input: &AppointmentCompletionInput,
    service_id: &str,
    now: &str,
    commission: f64,
) -> Result<(), String> {
    let member_id = appointment
        .1
        .as_deref()
        .ok_or_else(|| "套盒预约缺少会员信息".to_string())?;
    let purchase_id = appointment
        .5
        .as_deref()
        .ok_or_else(|| "套盒预约缺少套盒信息".to_string())?;
    let (package_name, remaining_uses): (String, i64) = transaction
        .query_row(
            "SELECT package_name,remaining_uses FROM package_purchases
             WHERE id=?1 AND member_id=?2 AND package_type='套盒'
               AND status='active' AND remaining_uses>0",
            params![purchase_id, member_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| "预约套盒已结束或没有剩余次数".to_string())?;
    let remaining_after = remaining_uses - 1;
    let changed = transaction
        .execute(
            "UPDATE package_purchases SET remaining_uses=?1,last_consumed_at=?2,
             status=CASE WHEN ?1=0 THEN 'completed' ELSE 'active' END
             WHERE id=?3 AND remaining_uses=?4 AND status='active'",
            params![remaining_after, now, purchase_id, remaining_uses],
        )
        .map_err(|error| error.to_string())?;
    if changed != 1 {
        return Err("套盒状态已变化，请刷新后重试".to_string());
    }
    transaction
        .execute(
            "INSERT INTO services
             (id,member_id,member_name,employee,service_name,service_type,duration,amount,
              commission,commission_rule_version,package_purchase_id,created_at,status)
             VALUES (?1,?2,?3,?4,?5,'套盒手工',?6,0,?7,2,?8,?9,'completed')",
            params![
                service_id,
                member_id,
                appointment.2,
                employee,
                package_name,
                input.duration,
                commission,
                purchase_id,
                now
            ],
        )
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "INSERT INTO package_consumptions
             (id,package_purchase_id,member_id,member_name,employee,package_name,consumed_at,
              remaining_after,commission,service_id,duration,note)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,'预约服务完成')",
            params![
                Uuid::new_v4().to_string(),
                purchase_id,
                member_id,
                appointment.2,
                employee,
                package_name,
                now,
                remaining_after,
                commission,
                service_id,
                input.duration
            ],
        )
        .map_err(|error| error.to_string())?;
    transaction
        .execute(
            "UPDATE members SET last_visit=?1 WHERE id=?2",
            params![now, member_id],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{time_ranges_overlap, transition_allowed};
    use chrono::{TimeZone, Utc};

    #[test]
    fn appointment_conflicts_only_when_time_ranges_overlap() {
        let start = Utc.with_ymd_and_hms(2026, 8, 20, 6, 0, 0).unwrap();
        assert!(time_ranges_overlap(
            start,
            60,
            start + chrono::Duration::minutes(30),
            60
        ));
        assert!(!time_ranges_overlap(
            start,
            60,
            start + chrono::Duration::minutes(60),
            60
        ));
    }

    #[test]
    fn appointment_status_transitions_are_restricted() {
        assert!(transition_allowed("pending", "arrived"));
        assert!(transition_allowed("arrived", "in_service"));
        assert!(!transition_allowed("completed", "arrived"));
        assert!(!transition_allowed("pending", "completed"));
    }
}
