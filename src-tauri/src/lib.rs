mod commands;
mod database;
mod models;
mod security;
mod state;

use commands::{
    appointments::{
        complete_appointment, create_appointment, set_appointment_status, update_appointment,
    },
    auth::{
        change_password, create_account, get_accounts, login, logout, reset_account_password,
        set_account_status,
    },
    backup::{automatic_daily_backup, create_backup, restore_latest_backup},
    packages::{
        consume_package, create_package, purchase_package, set_package_status, update_package,
    },
    payroll::{get_employee_salaries, update_commission_config, upsert_attendance},
    projects::{create_project, set_project_status, update_project},
    salon::{
        cancel_service, create_employee, create_member, create_service, create_transaction,
        get_snapshot, set_employee_status,
    },
};
use database::initialize_database;
use state::DatabaseState;
use std::{collections::HashMap, fs, sync::Mutex};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|error| error.to_string())?;
            fs::create_dir_all(&data_dir)?;
            let database_path = data_dir.join("salon.db");
            let backup_dir = data_dir.join("backups");
            let connection = initialize_database(&database_path).map_err(std::io::Error::other)?;
            automatic_daily_backup(&connection, &backup_dir).map_err(std::io::Error::other)?;
            app.manage(DatabaseState {
                connection: Mutex::new(connection),
                backup_dir,
                sessions: Mutex::new(HashMap::new()),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            login,
            logout,
            change_password,
            get_accounts,
            create_account,
            reset_account_password,
            set_account_status,
            get_snapshot,
            create_member,
            create_transaction,
            create_service,
            cancel_service,
            create_employee,
            set_employee_status,
            update_commission_config,
            upsert_attendance,
            get_employee_salaries,
            create_package,
            update_package,
            set_package_status,
            purchase_package,
            consume_package,
            create_project,
            update_project,
            set_project_status,
            create_appointment,
            update_appointment,
            set_appointment_status,
            complete_appointment,
            create_backup,
            restore_latest_backup
        ])
        .run(tauri::generate_context!())
        .expect("聚尚木子应用启动失败");
}
