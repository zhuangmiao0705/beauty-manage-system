use crate::models::AuthUser;
use rusqlite::Connection;
use std::{collections::HashMap, path::PathBuf, sync::Mutex};

pub(crate) struct DatabaseState {
    pub(crate) connection: Mutex<Connection>,
    pub(crate) backup_dir: PathBuf,
    pub(crate) sessions: Mutex<HashMap<String, AuthUser>>,
}
