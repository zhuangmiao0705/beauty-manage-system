use crate::{
    commands::auth::require_manager,
    database::{current_month, employee_compensation_for_name, round_money, snapshot},
    models::{
        AppSnapshot, AttendanceInput, CommissionConfigInput, EmployeeCompensation, EmployeeSalary,
    },
    state::DatabaseState,
};
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, TimeZone, Utc};
use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

const STANDARD_MONTHLY_REST_DAYS: i64 = 4;
const DEFAULT_ATTENDANCE_REST_DAYS: i64 = 0;

fn salary_components(
    compensation: &EmployeeCompensation,
    total_performance: f64,
    normal_service_count: i64,
    package_service_count: i64,
    work_days: i64,
    meal_days: i64,
    standard_work_days: i64,
) -> (f64, f64, f64, f64, f64) {
    let base_salary =
        round_money(compensation.base_salary * work_days as f64 / standard_work_days.max(1) as f64);
    let commission = round_money(
        total_performance * compensation.base_commission_rate
            + (total_performance - compensation.performance_target).max(0.0)
                * compensation.excess_commission_rate
            + normal_service_count as f64 * compensation.normal_service_commission
            + package_service_count as f64 * compensation.package_service_commission,
    );
    let meal_allowance = round_money(meal_days as f64 * compensation.meal_allowance_per_day);
    let attendance_bonus = if meal_days >= standard_work_days {
        round_money(compensation.attendance_bonus)
    } else {
        0.0
    };
    let total_income = round_money(base_salary + commission + meal_allowance + attendance_bonus);
    (
        base_salary,
        commission,
        meal_allowance,
        attendance_bonus,
        total_income,
    )
}

fn parse_month(month: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(&format!("{month}-01"), "%Y-%m-%d")
        .map_err(|_| "月份格式应为 YYYY-MM".to_string())
}

fn month_bounds(month: &str) -> Result<(DateTime<Utc>, DateTime<Utc>, i64), String> {
    let start_date = parse_month(month)?;
    let next_date = if start_date.month() == 12 {
        NaiveDate::from_ymd_opt(start_date.year() + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(start_date.year(), start_date.month() + 1, 1)
    }
    .ok_or_else(|| "月份无效".to_string())?;
    let start = Local
        .from_local_datetime(
            &start_date
                .and_hms_opt(0, 0, 0)
                .ok_or_else(|| "月份无效".to_string())?,
        )
        .single()
        .ok_or_else(|| "月份时区无效".to_string())?
        .with_timezone(&Utc);
    let end = Local
        .from_local_datetime(
            &next_date
                .and_hms_opt(0, 0, 0)
                .ok_or_else(|| "月份无效".to_string())?,
        )
        .single()
        .ok_or_else(|| "月份时区无效".to_string())?
        .with_timezone(&Utc);
    Ok((start, end, (next_date - start_date).num_days()))
}

#[tauri::command]
pub(crate) fn update_commission_config(
    token: String,
    input: CommissionConfigInput,
    state: tauri::State<DatabaseState>,
) -> Result<AppSnapshot, String> {
    require_manager(&state, &token)?;
    parse_month(&input.effective_month)?;
    if input.effective_month < current_month() {
        return Err("历史月份的提成配置不能修改".to_string());
    }
    if !(0.0..=1.0).contains(&input.recharge_rate)
        || !(0.0..=1.0).contains(&input.package_purchase_rate)
        || input.package_service_amount < 0.0
        || input.normal_service_amount < 0.0
        || input.base_salary < 0.0
    {
        return Err("提成比例、单笔提成或底薪无效".to_string());
    }
    let connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    let now = Utc::now().to_rfc3339();
    connection
        .execute(
            "INSERT INTO commission_configs
             (id,effective_month,recharge_rate,package_purchase_rate,package_service_commission,
              normal_service_commission,base_salary,created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)
             ON CONFLICT(effective_month) DO UPDATE SET
               recharge_rate=excluded.recharge_rate,
               package_purchase_rate=excluded.package_purchase_rate,
               package_service_commission=excluded.package_service_commission,
               normal_service_commission=excluded.normal_service_commission,
               base_salary=excluded.base_salary",
            params![
                Uuid::new_v4().to_string(),
                input.effective_month,
                input.recharge_rate,
                input.package_purchase_rate,
                round_money(input.package_service_amount),
                round_money(input.normal_service_amount),
                round_money(input.base_salary),
                now
            ],
        )
        .map_err(|error| error.to_string())?;
    snapshot(&connection)
}

#[tauri::command]
pub(crate) fn upsert_attendance(
    token: String,
    input: AttendanceInput,
    state: tauri::State<DatabaseState>,
) -> Result<AppSnapshot, String> {
    require_manager(&state, &token)?;
    let (_, _, days_in_month) = month_bounds(&input.month)?;
    if input.rest_days < 0 || input.rest_days > days_in_month {
        return Err("休息天数超出该月自然日范围".to_string());
    }
    let connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    let (employee_id, employee_name): (String, String) = connection
        .query_row(
            "SELECT id,name FROM employees WHERE id=?1",
            params![input.employee_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| "员工不存在".to_string())?;
    let now = Utc::now().to_rfc3339();
    connection
        .execute(
            "INSERT INTO attendance_records
             (id,employee_id,employee_name,month,rest_days,created_at,updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?6)
             ON CONFLICT(employee_id,month) DO UPDATE SET
               employee_name=excluded.employee_name,
               rest_days=excluded.rest_days,
               updated_at=excluded.updated_at",
            params![
                Uuid::new_v4().to_string(),
                employee_id,
                employee_name,
                input.month,
                input.rest_days,
                now
            ],
        )
        .map_err(|error| error.to_string())?;
    snapshot(&connection)
}

fn active_days_for_employee(
    connection: &rusqlite::Connection,
    employee_id: &str,
    start: DateTime<Utc>,
    days_in_month: i64,
) -> Result<i64, String> {
    let mut statement = connection
        .prepare(
            "SELECT status,effective_at FROM employee_status_events
             WHERE employee_id=?1 ORDER BY effective_at",
        )
        .map_err(|error| error.to_string())?;
    let events = statement
        .query_map(params![employee_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|(status, time)| {
            DateTime::parse_from_rfc3339(&time)
                .map(|value| (status, value.with_timezone(&Utc)))
                .map_err(|_| "员工状态时间无效".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut active_days = 0;
    for offset in 0..days_in_month {
        let day_end = start + Duration::days(offset + 1);
        let status = events
            .iter()
            .filter(|(_, effective_at)| *effective_at < day_end)
            .next_back()
            .map(|(status, _)| status.as_str());
        if status == Some("active") {
            active_days += 1;
        }
    }
    Ok(active_days)
}

#[tauri::command]
pub(crate) fn get_employee_salaries(
    token: String,
    month: String,
    employee_name: Option<String>,
    state: tauri::State<DatabaseState>,
) -> Result<Vec<EmployeeSalary>, String> {
    require_manager(&state, &token)?;
    let (start, end, days_in_month) = month_bounds(&month)?;
    let connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    let start_text = start.to_rfc3339();
    let end_text = end.to_rfc3339();
    let mut employee_statement = connection
        .prepare(
            "SELECT id,name FROM employees WHERE created_at<?1
             AND (?2='' OR name LIKE '%'||?2||'%') ORDER BY name",
        )
        .map_err(|error| error.to_string())?;
    let employees = employee_statement
        .query_map(
            params![end_text, employee_name.unwrap_or_default().trim()],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let standard_work_days = (days_in_month - STANDARD_MONTHLY_REST_DAYS).max(1);
    let elapsed_days = match month.as_str().cmp(current_month().as_str()) {
        std::cmp::Ordering::Less => days_in_month,
        std::cmp::Ordering::Equal => Local::now().day() as i64,
        std::cmp::Ordering::Greater => 0,
    };
    let mut salaries = Vec::new();
    for (employee_id, name) in employees {
        let compensation = employee_compensation_for_name(&connection, &name)?;
        let recharge_amount: f64 = connection
            .query_row(
                "SELECT COALESCE(SUM(amount),0) FROM transactions
                 WHERE type='recharge' AND employee=?1 AND created_at>=?2 AND created_at<?3",
                params![name, start_text, end_text],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        let package_purchase_amount: f64 = connection
            .query_row(
                "SELECT COALESCE(SUM(cash_payment_amount),0) FROM package_purchases
                 WHERE employee=?1 AND purchased_at>=?2 AND purchased_at<?3",
                params![name, start_text, end_text],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        let normal_service_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM services
                 WHERE employee=?1 AND service_type='普通手工' AND status='completed'
                   AND created_at>=?2 AND created_at<?3",
                params![name, start_text, end_text],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        let package_service_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM services
                 WHERE employee=?1 AND service_type='套盒手工' AND status='completed'
                   AND created_at>=?2 AND created_at<?3",
                params![name, start_text, end_text],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        let rest_days = connection
            .query_row(
                "SELECT rest_days FROM attendance_records WHERE employee_id=?1 AND month=?2",
                params![employee_id, month],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        let rest_days = rest_days.unwrap_or(DEFAULT_ATTENDANCE_REST_DAYS);
        let active_days =
            active_days_for_employee(&connection, &employee_id, start, days_in_month)?;
        let elapsed_active_days =
            active_days_for_employee(&connection, &employee_id, start, elapsed_days)?;
        let work_days = (active_days - rest_days).max(0);
        let meal_days = (elapsed_active_days - rest_days).max(0);
        let total_performance = round_money(recharge_amount + package_purchase_amount);
        let (base_salary, commission, meal_allowance, attendance_bonus, total_income) =
            salary_components(
                &compensation,
                total_performance,
                normal_service_count,
                package_service_count,
                work_days,
                meal_days,
                standard_work_days,
            );
        salaries.push(EmployeeSalary {
            employee_id,
            name,
            recharge_performance: round_money(recharge_amount),
            package_purchase_performance: round_money(package_purchase_amount),
            total_performance,
            package_service_count,
            normal_service_count,
            commission,
            base_salary,
            meal_allowance,
            attendance_bonus,
            total_income,
            rest_days,
            active_days,
            work_days,
            meal_days,
        });
    }
    Ok(salaries)
}

#[cfg(test)]
mod tests {
    use super::salary_components;
    use crate::models::EmployeeCompensation;

    fn compensation() -> EmployeeCompensation {
        EmployeeCompensation {
            employee_id: "e1".to_string(),
            base_salary: 1800.0,
            base_commission_rate: 0.10,
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
    fn salary_uses_performance_service_meal_and_attendance_rules() {
        assert_eq!(
            salary_components(&compensation(), 13000.0, 2, 1, 27, 27, 27),
            (1800.0, 1380.0, 270.0, 300.0, 3750.0)
        );
        assert_eq!(
            salary_components(&compensation(), 13000.0, 2, 1, 27, 13, 27),
            (1800.0, 1380.0, 130.0, 0.0, 3310.0)
        );
    }
}
