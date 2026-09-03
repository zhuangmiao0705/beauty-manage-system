use crate::{
    database::{auth_user_from_row, list_accounts},
    models::{AccountInput, AccountRecord, AuthResponse, AuthUser},
    security::{hash_password, verify_password},
    state::DatabaseState,
};
use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

pub(crate) fn require_session(
    state: &tauri::State<DatabaseState>,
    token: &str,
) -> Result<AuthUser, String> {
    state
        .sessions
        .lock()
        .map_err(|_| "会话读取失败".to_string())?
        .get(token)
        .cloned()
        .ok_or_else(|| "登录已过期，请重新登录".to_string())
}

pub(crate) fn require_manager(
    state: &tauri::State<DatabaseState>,
    token: &str,
) -> Result<AuthUser, String> {
    let user = require_session(state, token)?;
    if user.role != "manager" {
        return Err("当前账号没有店长权限".to_string());
    }
    Ok(user)
}

pub(crate) fn revoke_account_sessions(
    state: &tauri::State<DatabaseState>,
    account_id: &str,
) -> Result<(), String> {
    state
        .sessions
        .lock()
        .map_err(|_| "会话更新失败".to_string())?
        .retain(|_, user| user.id != account_id);
    Ok(())
}

#[tauri::command]
pub(crate) fn login(
    username: String,
    password: String,
    state: tauri::State<DatabaseState>,
) -> Result<AuthResponse, String> {
    let connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    let account = connection
        .query_row(
            "SELECT id,username,display_name,role,employee_id,status,must_change_password,password_hash
             FROM accounts WHERE username=?1",
            params![username.trim()],
            |row| {
                Ok((
                    auth_user_from_row(row)?,
                    row.get::<_, String>(7)?,
                ))
            },
        )
        .optional()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "账号或密码错误".to_string())?;
    if !verify_password(&password, &account.1) {
        return Err("账号或密码错误".to_string());
    }
    if account.0.status != "active" {
        return Err("该账号已停用，请联系店长".to_string());
    }
    drop(connection);

    let token = Uuid::new_v4().to_string();
    state
        .sessions
        .lock()
        .map_err(|_| "会话创建失败".to_string())?
        .insert(token.clone(), account.0.clone());
    Ok(AuthResponse {
        token,
        user: account.0,
    })
}

#[tauri::command]
pub(crate) fn logout(token: String, state: tauri::State<DatabaseState>) -> Result<(), String> {
    state
        .sessions
        .lock()
        .map_err(|_| "会话退出失败".to_string())?
        .remove(&token);
    Ok(())
}

#[tauri::command]
pub(crate) fn change_password(
    token: String,
    current_password: String,
    new_password: String,
    state: tauri::State<DatabaseState>,
) -> Result<AuthUser, String> {
    let mut user = require_session(&state, &token)?;
    if new_password.len() < 6 {
        return Err("新密码至少需要 6 位".to_string());
    }
    let connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    let current_hash: String = connection
        .query_row(
            "SELECT password_hash FROM accounts WHERE id=?1",
            params![user.id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if !verify_password(&current_password, &current_hash) {
        return Err("当前密码错误".to_string());
    }
    let now = Utc::now().to_rfc3339();
    connection
        .execute(
            "UPDATE accounts SET password_hash=?1,must_change_password=0,updated_at=?2 WHERE id=?3",
            params![hash_password(&new_password)?, now, user.id],
        )
        .map_err(|error| error.to_string())?;
    drop(connection);
    user.must_change_password = false;
    state
        .sessions
        .lock()
        .map_err(|_| "会话更新失败".to_string())?
        .insert(token, user.clone());
    Ok(user)
}

#[tauri::command]
pub(crate) fn get_accounts(
    token: String,
    state: tauri::State<DatabaseState>,
) -> Result<Vec<AccountRecord>, String> {
    require_manager(&state, &token)?;
    let connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    list_accounts(&connection)
}

#[tauri::command]
pub(crate) fn create_account(
    token: String,
    input: AccountInput,
    state: tauri::State<DatabaseState>,
) -> Result<Vec<AccountRecord>, String> {
    require_manager(&state, &token)?;
    if input.username.trim().len() < 3 || input.password.len() < 6 {
        return Err("登录账号至少 3 位，初始密码至少 6 位".to_string());
    }
    if input.role != "manager" && input.role != "employee" {
        return Err("账号角色无效".to_string());
    }
    let connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    let (display_name, employee_id) = if input.role == "employee" {
        if let Some(employee_id) = input.employee_id.filter(|value| !value.trim().is_empty()) {
            let employee_name = connection
                .query_row(
                    "SELECT name FROM employees WHERE id=?1 AND status='active'",
                    params![employee_id],
                    |row| row.get::<_, String>(0),
                )
                .map_err(|_| "请选择在职员工档案".to_string())?;
            (employee_name, Some(employee_id))
        } else {
            if input.display_name.trim().is_empty() {
                return Err("请输入共用账号显示名称".to_string());
            }
            (input.display_name.trim().to_string(), None)
        }
    } else {
        if input.display_name.trim().is_empty() {
            return Err("请输入账号显示姓名".to_string());
        }
        (input.display_name.trim().to_string(), None)
    };
    let now = Utc::now().to_rfc3339();
    connection
        .execute(
            "INSERT INTO accounts
             (id,username,password_hash,display_name,role,employee_id,status,must_change_password,created_at,updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,'active',0,?7,?7)",
            params![
                Uuid::new_v4().to_string(),
                input.username.trim(),
                hash_password(&input.password)?,
                display_name,
                input.role,
                employee_id,
                now
            ],
        )
        .map_err(|error| {
            if error.to_string().contains("UNIQUE") {
                "登录账号已存在".to_string()
            } else {
                error.to_string()
            }
        })?;
    list_accounts(&connection)
}

#[tauri::command]
pub(crate) fn reset_account_password(
    token: String,
    account_id: String,
    new_password: String,
    state: tauri::State<DatabaseState>,
) -> Result<Vec<AccountRecord>, String> {
    let manager = require_manager(&state, &token)?;
    if new_password.len() < 6 {
        return Err("新密码至少需要 6 位".to_string());
    }
    if manager.id == account_id {
        return Err("当前登录账号请在“个人账号”中修改密码".to_string());
    }
    let connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    connection
        .execute(
            "UPDATE accounts SET password_hash=?1,must_change_password=0,updated_at=?2 WHERE id=?3",
            params![
                hash_password(&new_password)?,
                Utc::now().to_rfc3339(),
                account_id
            ],
        )
        .map_err(|error| error.to_string())?;
    let accounts = list_accounts(&connection)?;
    drop(connection);
    revoke_account_sessions(&state, &account_id)?;
    Ok(accounts)
}

#[tauri::command]
pub(crate) fn set_account_status(
    token: String,
    account_id: String,
    status: String,
    state: tauri::State<DatabaseState>,
) -> Result<Vec<AccountRecord>, String> {
    let manager = require_manager(&state, &token)?;
    if status != "active" && status != "inactive" {
        return Err("账号状态无效".to_string());
    }
    if manager.id == account_id && status == "inactive" {
        return Err("不能停用当前登录账号".to_string());
    }
    let connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    let target_role: String = connection
        .query_row(
            "SELECT role FROM accounts WHERE id=?1",
            params![account_id],
            |row| row.get(0),
        )
        .map_err(|_| "账号不存在".to_string())?;
    if target_role == "manager" && status == "inactive" {
        let active_managers: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM accounts WHERE role='manager' AND status='active'",
                [],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        if active_managers <= 1 {
            return Err("系统至少需要保留一个启用的店长账号".to_string());
        }
    }
    connection
        .execute(
            "UPDATE accounts SET status=?1,updated_at=?2 WHERE id=?3",
            params![status, Utc::now().to_rfc3339(), account_id],
        )
        .map_err(|error| error.to_string())?;
    let accounts = list_accounts(&connection)?;
    drop(connection);
    if status == "inactive" {
        revoke_account_sessions(&state, &account_id)?;
    }
    Ok(accounts)
}
