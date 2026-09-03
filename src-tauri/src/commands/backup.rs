use crate::{
    commands::auth::require_manager,
    database::{migrate_database, seed_default_manager, seed_employees, snapshot},
    models::AppSnapshot,
    state::DatabaseState,
};
use chrono::Local;
use rusqlite::{backup::Backup, Connection};
use std::{fs, path::PathBuf, time::Duration};

pub(crate) fn backup_to(
    connection: &Connection,
    backup_dir: &PathBuf,
    label: &str,
) -> Result<PathBuf, String> {
    fs::create_dir_all(backup_dir).map_err(|error| error.to_string())?;
    let path = backup_dir.join(format!(
        "salon-{}-{}.db",
        label,
        Local::now().format("%Y%m%d-%H%M%S")
    ));
    let mut destination = Connection::open(&path).map_err(|error| error.to_string())?;
    let backup = Backup::new(connection, &mut destination).map_err(|error| error.to_string())?;
    backup
        .run_to_completion(5, Duration::from_millis(50), None)
        .map_err(|error| error.to_string())?;
    drop(backup);
    destination
        .execute_batch("PRAGMA integrity_check;")
        .map_err(|error| error.to_string())?;
    Ok(path)
}

pub(crate) fn automatic_daily_backup(
    connection: &Connection,
    backup_dir: &PathBuf,
) -> Result<(), String> {
    fs::create_dir_all(backup_dir).map_err(|error| error.to_string())?;
    let today = Local::now().format("%Y%m%d").to_string();
    let already_backed_up = fs::read_dir(backup_dir)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .any(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .contains(&format!("auto-{today}"))
        });
    if !already_backed_up {
        backup_to(connection, backup_dir, "auto")?;
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn create_backup(
    token: String,
    state: tauri::State<DatabaseState>,
) -> Result<String, String> {
    require_manager(&state, &token)?;
    let connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    let path = backup_to(&connection, &state.backup_dir, "manual")?;
    let mut files = fs::read_dir(&state.backup_dir)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    files.sort_by_key(|entry| {
        entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .ok()
    });
    if files.len() > 30 {
        for entry in files.iter().take(files.len() - 30) {
            let _ = fs::remove_file(entry.path());
        }
    }
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub(crate) fn restore_latest_backup(
    token: String,
    state: tauri::State<DatabaseState>,
) -> Result<AppSnapshot, String> {
    require_manager(&state, &token)?;
    let mut files = fs::read_dir(&state.backup_dir)
        .map_err(|_| "还没有可恢复的备份".to_string())?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "db")
        })
        .collect::<Vec<_>>();
    files.sort_by_key(|entry| {
        entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .ok()
    });
    let latest = files
        .last()
        .ok_or_else(|| "还没有可恢复的备份".to_string())?
        .path();
    let mut connection = state
        .connection
        .lock()
        .map_err(|_| "数据库锁定失败".to_string())?;
    backup_to(&connection, &state.backup_dir, "before-restore")?;
    let source = Connection::open(latest).map_err(|error| error.to_string())?;
    let backup = Backup::new(&source, &mut connection).map_err(|error| error.to_string())?;
    backup
        .run_to_completion(5, Duration::from_millis(50), None)
        .map_err(|error| error.to_string())?;
    drop(backup);
    migrate_database(&connection)?;
    seed_employees(&connection)?;
    seed_default_manager(&connection)?;
    snapshot(&connection)
}
