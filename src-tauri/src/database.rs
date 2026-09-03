use crate::{models::*, security::hash_password};
use chrono::{Local, Utc};
use rusqlite::{params, Connection, Row};
use std::path::PathBuf;
use uuid::Uuid;

fn column_exists(connection: &Connection, table: &str, column: &str) -> Result<bool, String> {
    connection
        .query_row(
            &format!("SELECT COUNT(*) FROM pragma_table_info('{table}') WHERE name=?1"),
            params![column],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count > 0)
        .map_err(|error| error.to_string())
}

fn add_column(connection: &Connection, table: &str, definition: &str) -> Result<(), String> {
    let column = definition.split_whitespace().next().unwrap_or_default();
    if !column_exists(connection, table, column)? {
        connection
            .execute(&format!("ALTER TABLE {table} ADD COLUMN {definition}"), [])
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[derive(Debug)]
struct LegacyBalanceEvent {
    record_type: String,
    amount: f64,
    gift_amount: f64,
    balance_after: f64,
    payment_method: String,
    item: String,
}

fn split_legacy_member_balance(current_balance: f64, records: &[LegacyBalanceEvent]) -> (f64, f64) {
    let Some(first) = records.first() else {
        return (round_money(current_balance), 0.0);
    };
    let opening_balance = if first.record_type == "recharge" {
        first.balance_after - first.amount - first.gift_amount
    } else if first.payment_method == "会员余额" {
        first.balance_after + first.amount
    } else {
        first.balance_after
    };
    let mut principal_balance = round_money(opening_balance.max(0.0));
    let mut gift_balance = 0.0;

    for record in records {
        if record.record_type == "recharge" {
            principal_balance = round_money(principal_balance + record.amount);
            gift_balance = round_money(gift_balance + record.gift_amount);
            continue;
        }
        if record.payment_method != "会员余额" {
            continue;
        }
        if record.item.starts_with("套盒购买：") || record.item.starts_with("套餐购买：")
        {
            let principal_deduction = principal_balance.min(record.amount);
            principal_balance = round_money(principal_balance - principal_deduction);
            gift_balance =
                round_money((gift_balance - (record.amount - principal_deduction)).max(0.0));
            continue;
        }
        let gift_deduction = gift_balance.min(record.amount);
        gift_balance = round_money(gift_balance - gift_deduction);
        principal_balance =
            round_money((principal_balance - (record.amount - gift_deduction)).max(0.0));
    }

    let difference = round_money(current_balance - principal_balance - gift_balance);
    if difference >= 0.0 {
        principal_balance = round_money(principal_balance + difference);
    } else {
        let deficit = -difference;
        let gift_reduction = gift_balance.min(deficit);
        gift_balance = round_money(gift_balance - gift_reduction);
        principal_balance = round_money((principal_balance - (deficit - gift_reduction)).max(0.0));
    }
    (principal_balance, gift_balance)
}

fn backfill_member_balance_parts(connection: &Connection) -> Result<(), String> {
    let members = {
        let mut statement = connection
            .prepare("SELECT id,balance FROM members")
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
            })
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        rows
    };
    for (member_id, current_balance) in members {
        let records = {
            let mut statement = connection
                .prepare(
                    "SELECT type,amount,gift_amount,balance_after,payment_method,item
                     FROM transactions WHERE member_id=?1 ORDER BY created_at,id",
                )
                .map_err(|error| error.to_string())?;
            let rows = statement
                .query_map(params![member_id], |row| {
                    Ok(LegacyBalanceEvent {
                        record_type: row.get(0)?,
                        amount: row.get(1)?,
                        gift_amount: row.get(2)?,
                        balance_after: row.get(3)?,
                        payment_method: row.get(4)?,
                        item: row.get(5)?,
                    })
                })
                .map_err(|error| error.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())?;
            rows
        };
        let (principal_balance, gift_balance) =
            split_legacy_member_balance(current_balance, &records);
        connection
            .execute(
                "UPDATE members SET principal_balance=?1,gift_balance=?2 WHERE id=?3",
                params![principal_balance, gift_balance, member_id],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub(crate) fn current_month() -> String {
    Local::now().format("%Y-%m").to_string()
}

pub(crate) fn round_money(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

pub(crate) fn validate_active_employee(
    connection: &Connection,
    employee: &str,
) -> Result<String, String> {
    let name = employee.trim();
    if name.is_empty() {
        return Err("请选择经办员工".to_string());
    }
    connection
        .query_row(
            "SELECT name FROM employees WHERE name=?1 AND status='active'",
            params![name],
            |row| row.get::<_, String>(0),
        )
        .map_err(|_| "没有找到启用状态的员工".to_string())
}

pub(crate) fn employee_compensation_for_name(
    connection: &Connection,
    employee: &str,
) -> Result<EmployeeCompensation, String> {
    connection
        .query_row(
            "SELECT c.employee_id,c.base_salary,c.base_commission_rate,c.performance_target,
                    c.excess_commission_rate,c.meal_allowance_per_day,c.attendance_bonus,
                    c.normal_service_commission,c.package_service_commission,c.created_at
             FROM employee_compensations c JOIN employees e ON e.id=c.employee_id
             WHERE e.name=?1",
            params![employee],
            |row| {
                Ok(EmployeeCompensation {
                    employee_id: row.get(0)?,
                    base_salary: row.get(1)?,
                    base_commission_rate: row.get(2)?,
                    performance_target: row.get(3)?,
                    excess_commission_rate: row.get(4)?,
                    meal_allowance_per_day: row.get(5)?,
                    attendance_bonus: row.get(6)?,
                    normal_service_commission: row.get(7)?,
                    package_service_commission: row.get(8)?,
                    created_at: row.get(9)?,
                })
            },
        )
        .map_err(|_| "没有找到员工薪酬配置".to_string())
}

pub(crate) fn migrate_database(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=FULL;
             PRAGMA foreign_keys=ON;
             PRAGMA busy_timeout=5000;

             CREATE TABLE IF NOT EXISTS schema_migrations (
               version INTEGER PRIMARY KEY,
               applied_at TEXT NOT NULL
             );

             CREATE TABLE IF NOT EXISTS employees (
               id TEXT PRIMARY KEY,
               name TEXT NOT NULL UNIQUE,
               role TEXT NOT NULL,
               commission_rate REAL NOT NULL DEFAULT 0,
               status TEXT NOT NULL DEFAULT 'active',
               color TEXT NOT NULL,
               created_at TEXT NOT NULL DEFAULT ''
             );

             CREATE TABLE IF NOT EXISTS members (
               id TEXT PRIMARY KEY,
               name TEXT NOT NULL,
               phone TEXT NOT NULL,
               balance REAL NOT NULL DEFAULT 0 CHECK(balance >= 0),
               principal_balance REAL NOT NULL DEFAULT 0 CHECK(principal_balance >= 0),
               gift_balance REAL NOT NULL DEFAULT 0 CHECK(gift_balance >= 0),
               total_recharge REAL NOT NULL DEFAULT 0,
               total_consumption REAL NOT NULL DEFAULT 0,
               join_date TEXT NOT NULL,
               last_visit TEXT NOT NULL,
               status TEXT NOT NULL DEFAULT 'active'
             );

             CREATE TABLE IF NOT EXISTS transactions (
               id TEXT PRIMARY KEY,
               member_id TEXT NOT NULL REFERENCES members(id),
               member_name TEXT NOT NULL,
               type TEXT NOT NULL CHECK(type IN ('recharge', 'consume')),
               amount REAL NOT NULL CHECK(amount > 0),
               balance_after REAL NOT NULL,
               payment_method TEXT NOT NULL,
               item TEXT NOT NULL,
               employee TEXT NOT NULL,
               created_at TEXT NOT NULL,
               note TEXT NOT NULL DEFAULT '',
               gift_amount REAL NOT NULL DEFAULT 0,
               commission REAL NOT NULL DEFAULT 0,
               commission_rule_version INTEGER NOT NULL DEFAULT 2
             );

             CREATE TABLE IF NOT EXISTS services (
               id TEXT PRIMARY KEY,
               member_id TEXT REFERENCES members(id),
               member_name TEXT NOT NULL,
               employee TEXT NOT NULL,
               service_name TEXT NOT NULL,
               service_type TEXT NOT NULL DEFAULT '普通手工' CHECK(service_type IN ('套盒手工', '普通手工')),
               duration INTEGER NOT NULL CHECK(duration > 0),
               amount REAL NOT NULL CHECK(amount >= 0),
               commission REAL NOT NULL DEFAULT 0,
               commission_rule_version INTEGER NOT NULL DEFAULT 2,
               created_at TEXT NOT NULL,
               status TEXT NOT NULL DEFAULT 'completed',
               package_purchase_id TEXT
             );

             CREATE TABLE IF NOT EXISTS accounts (
               id TEXT PRIMARY KEY,
               username TEXT NOT NULL UNIQUE,
               password_hash TEXT NOT NULL,
               display_name TEXT NOT NULL,
               role TEXT NOT NULL CHECK(role IN ('manager', 'employee')),
               employee_id TEXT REFERENCES employees(id),
               status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'inactive')),
               must_change_password INTEGER NOT NULL DEFAULT 0,
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL
             );

             CREATE TABLE IF NOT EXISTS employee_status_events (
               id TEXT PRIMARY KEY,
               employee_id TEXT NOT NULL REFERENCES employees(id),
               employee_name TEXT NOT NULL,
               status TEXT NOT NULL CHECK(status IN ('active','inactive')),
               effective_at TEXT NOT NULL
             );

             CREATE TABLE IF NOT EXISTS employee_compensations (
               employee_id TEXT PRIMARY KEY REFERENCES employees(id),
               base_salary REAL NOT NULL CHECK(base_salary>=0),
               base_commission_rate REAL NOT NULL CHECK(base_commission_rate>=0 AND base_commission_rate<=1),
               performance_target REAL NOT NULL CHECK(performance_target>=0),
               excess_commission_rate REAL NOT NULL CHECK(excess_commission_rate>=0 AND excess_commission_rate<=1),
               meal_allowance_per_day REAL NOT NULL CHECK(meal_allowance_per_day>=0),
               attendance_bonus REAL NOT NULL CHECK(attendance_bonus>=0),
               normal_service_commission REAL NOT NULL CHECK(normal_service_commission>=0),
               package_service_commission REAL NOT NULL CHECK(package_service_commission>=0),
               created_at TEXT NOT NULL
             );

             CREATE TABLE IF NOT EXISTS commission_configs (
               id TEXT PRIMARY KEY,
               effective_month TEXT NOT NULL UNIQUE,
               recharge_rate REAL NOT NULL CHECK(recharge_rate>=0),
               package_purchase_rate REAL NOT NULL CHECK(package_purchase_rate>=0),
               package_service_commission REAL NOT NULL CHECK(package_service_commission>=0),
               normal_service_commission REAL NOT NULL CHECK(normal_service_commission>=0),
               base_salary REAL NOT NULL CHECK(base_salary>=0),
               created_at TEXT NOT NULL
             );

             CREATE TABLE IF NOT EXISTS attendance_records (
               id TEXT PRIMARY KEY,
               employee_id TEXT NOT NULL REFERENCES employees(id),
               employee_name TEXT NOT NULL,
               month TEXT NOT NULL,
               rest_days INTEGER NOT NULL CHECK(rest_days>=0 AND rest_days<=31),
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL,
               UNIQUE(employee_id,month)
             );

             CREATE TABLE IF NOT EXISTS packages (
               id TEXT PRIMARY KEY,
               name TEXT NOT NULL UNIQUE,
               price REAL NOT NULL CHECK(price>0),
               total_uses INTEGER NOT NULL CHECK(total_uses>0),
               package_type TEXT NOT NULL DEFAULT '套盒' CHECK(package_type IN ('套盒','普通')),
               status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active','inactive')),
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL
             );

             CREATE TABLE IF NOT EXISTS package_purchases (
               id TEXT PRIMARY KEY,
               member_id TEXT NOT NULL REFERENCES members(id),
               member_name TEXT NOT NULL,
               employee TEXT NOT NULL,
               package_id TEXT NOT NULL REFERENCES packages(id),
               package_name TEXT NOT NULL,
               package_type TEXT NOT NULL DEFAULT '套盒' CHECK(package_type IN ('套盒','普通')),
               price REAL NOT NULL CHECK(price>0),
               total_uses INTEGER NOT NULL CHECK(total_uses>0),
               remaining_uses INTEGER NOT NULL CHECK(remaining_uses>=0),
               payment_method TEXT NOT NULL CHECK(payment_method IN ('会员余额','现金','余额现金组合支付')),
               balance_payment_amount REAL NOT NULL DEFAULT 0 CHECK(balance_payment_amount>=0),
               cash_payment_amount REAL NOT NULL DEFAULT 0 CHECK(cash_payment_amount>=0),
               status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active','completed')),
               purchased_at TEXT NOT NULL,
               last_consumed_at TEXT,
               commission REAL NOT NULL DEFAULT 0,
               commission_rule_version INTEGER NOT NULL DEFAULT 2,
               transaction_id TEXT NOT NULL DEFAULT ''
             );

             CREATE TABLE IF NOT EXISTS package_consumptions (
               id TEXT PRIMARY KEY,
               package_purchase_id TEXT NOT NULL REFERENCES package_purchases(id),
               member_id TEXT NOT NULL REFERENCES members(id),
               member_name TEXT NOT NULL,
               employee TEXT NOT NULL,
               package_name TEXT NOT NULL,
               consumed_at TEXT NOT NULL,
               remaining_after INTEGER NOT NULL CHECK(remaining_after>=0),
               commission REAL NOT NULL DEFAULT 0,
               service_id TEXT NOT NULL REFERENCES services(id),
               duration INTEGER NOT NULL DEFAULT 60,
               note TEXT NOT NULL DEFAULT ''
             );

             CREATE INDEX IF NOT EXISTS idx_transactions_member ON transactions(member_id);
             CREATE INDEX IF NOT EXISTS idx_transactions_created ON transactions(created_at);
             CREATE INDEX IF NOT EXISTS idx_transactions_employee ON transactions(employee,created_at);
             CREATE INDEX IF NOT EXISTS idx_services_member ON services(member_id);
             CREATE INDEX IF NOT EXISTS idx_services_created ON services(created_at);
             CREATE INDEX IF NOT EXISTS idx_services_employee ON services(employee,created_at);
             CREATE INDEX IF NOT EXISTS idx_accounts_employee ON accounts(employee_id);
             CREATE INDEX IF NOT EXISTS idx_employee_events ON employee_status_events(employee_id,effective_at);
             CREATE INDEX IF NOT EXISTS idx_attendance_month ON attendance_records(month,employee_id);
             CREATE INDEX IF NOT EXISTS idx_package_purchases_member ON package_purchases(member_id,purchased_at);
             CREATE INDEX IF NOT EXISTS idx_package_purchases_employee ON package_purchases(employee,purchased_at);
             CREATE INDEX IF NOT EXISTS idx_package_consumptions_purchase ON package_consumptions(package_purchase_id,consumed_at);",
        )
        .map_err(|error| error.to_string())?;

    let service_member_required = connection
        .query_row(
            "SELECT \"notnull\" FROM pragma_table_info('services') WHERE name='member_id'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or_default();
    if service_member_required != 0 {
        connection
            .execute_batch(
                "PRAGMA foreign_keys=OFF;
                 BEGIN;
                 ALTER TABLE services RENAME TO services_legacy;
                 CREATE TABLE services (
                   id TEXT PRIMARY KEY,
                   member_id TEXT REFERENCES members(id), member_name TEXT NOT NULL,
                   employee TEXT NOT NULL, service_name TEXT NOT NULL,
                   service_type TEXT NOT NULL DEFAULT '普通手工' CHECK(service_type IN ('套盒手工','普通手工')),
                   duration INTEGER NOT NULL CHECK(duration>0), amount REAL NOT NULL CHECK(amount>=0),
                   commission REAL NOT NULL DEFAULT 0,
                   commission_rule_version INTEGER NOT NULL DEFAULT 1,
                   created_at TEXT NOT NULL,
                   status TEXT NOT NULL DEFAULT 'completed', package_purchase_id TEXT
                 );
                 INSERT INTO services
                   (id,member_id,member_name,employee,service_name,duration,amount,commission,created_at,status)
                 SELECT id,member_id,member_name,employee,service_name,duration,amount,commission,created_at,status
                 FROM services_legacy;
                 DROP TABLE services_legacy;
                 DROP TABLE package_consumptions;
                 CREATE TABLE package_consumptions (
                   id TEXT PRIMARY KEY,
                   package_purchase_id TEXT NOT NULL REFERENCES package_purchases(id),
                   member_id TEXT NOT NULL REFERENCES members(id),
                   member_name TEXT NOT NULL,
                   employee TEXT NOT NULL,
                   package_name TEXT NOT NULL,
                   consumed_at TEXT NOT NULL,
                   remaining_after INTEGER NOT NULL CHECK(remaining_after>=0),
                   commission REAL NOT NULL DEFAULT 0,
                   service_id TEXT NOT NULL REFERENCES services(id),
                   duration INTEGER NOT NULL DEFAULT 60,
                   note TEXT NOT NULL DEFAULT ''
                 );
                 COMMIT;
                 PRAGMA foreign_keys=ON;",
            )
            .map_err(|error| error.to_string())?;
    }
    add_column(
        connection,
        "services",
        "service_type TEXT NOT NULL DEFAULT '普通手工' CHECK(service_type IN ('套盒手工', '普通手工'))",
    )?;
    add_column(connection, "services", "package_purchase_id TEXT")?;
    add_column(
        connection,
        "employees",
        "created_at TEXT NOT NULL DEFAULT ''",
    )?;
    add_column(
        connection,
        "members",
        "principal_balance REAL NOT NULL DEFAULT 0",
    )?;
    add_column(
        connection,
        "members",
        "gift_balance REAL NOT NULL DEFAULT 0",
    )?;
    add_column(
        connection,
        "transactions",
        "gift_amount REAL NOT NULL DEFAULT 0",
    )?;
    add_column(
        connection,
        "transactions",
        "commission REAL NOT NULL DEFAULT 0",
    )?;
    add_column(
        connection,
        "transactions",
        "commission_rule_version INTEGER NOT NULL DEFAULT 1",
    )?;
    add_column(
        connection,
        "services",
        "commission_rule_version INTEGER NOT NULL DEFAULT 1",
    )?;
    add_column(
        connection,
        "package_purchases",
        "transaction_id TEXT NOT NULL DEFAULT ''",
    )?;
    add_column(
        connection,
        "package_purchases",
        "commission_rule_version INTEGER NOT NULL DEFAULT 1",
    )?;
    add_column(
        connection,
        "package_consumptions",
        "duration INTEGER NOT NULL DEFAULT 60",
    )?;
    add_column(
        connection,
        "package_consumptions",
        "note TEXT NOT NULL DEFAULT ''",
    )?;
    connection
        .execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_services_member ON services(member_id);
             CREATE INDEX IF NOT EXISTS idx_services_created ON services(created_at);
             CREATE INDEX IF NOT EXISTS idx_services_employee ON services(employee,created_at);
             CREATE INDEX IF NOT EXISTS idx_package_consumptions_purchase
               ON package_consumptions(package_purchase_id,consumed_at);",
        )
        .map_err(|error| error.to_string())?;

    let now = Utc::now().to_rfc3339();
    connection
        .execute(
            "INSERT OR IGNORE INTO commission_configs
             (id,effective_month,recharge_rate,package_purchase_rate,package_service_commission,
              normal_service_commission,base_salary,created_at)
             VALUES (?1,'1970-01',0.10,0.10,10,5,1800,?2)",
            params![Uuid::new_v4().to_string(), now],
        )
        .map_err(|error| error.to_string())?;

    let migration_applied: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version=3",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if migration_applied == 0 {
        let transaction = connection
            .unchecked_transaction()
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "UPDATE employees SET created_at=COALESCE(
                   (SELECT MIN(recorded_at) FROM (
                      SELECT created_at AS recorded_at FROM transactions
                        WHERE employee=employees.name
                      UNION ALL
                      SELECT created_at AS recorded_at FROM services
                        WHERE employee=employees.name
                      UNION ALL
                      SELECT created_at AS recorded_at FROM accounts
                        WHERE employee_id=employees.id
                   )),?1)
                 WHERE created_at=''",
                params![now],
            )
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "UPDATE transactions SET commission=ROUND(amount*0.10,2)
                 WHERE type='recharge' AND commission=0",
                [],
            )
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "UPDATE services SET commission=CASE service_type WHEN '套盒手工' THEN 10 ELSE 5 END
                 WHERE commission=0 AND status='completed'",
                [],
            )
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "INSERT INTO employee_status_events(id,employee_id,employee_name,status,effective_at)
                 SELECT lower(hex(randomblob(16))),id,name,'active',created_at FROM employees e
                 WHERE NOT EXISTS (SELECT 1 FROM employee_status_events x WHERE x.employee_id=e.id)",
                [],
            )
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "INSERT INTO employee_status_events(id,employee_id,employee_name,status,effective_at)
                 SELECT lower(hex(randomblob(16))),id,name,'inactive',?1 FROM employees e
                 WHERE status='inactive'
                   AND NOT EXISTS (
                     SELECT 1 FROM employee_status_events x
                     WHERE x.employee_id=e.id AND x.status='inactive'
                   )",
                params![now],
            )
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "INSERT INTO schema_migrations(version,applied_at) VALUES (3,?1)",
                params![now],
            )
            .map_err(|error| error.to_string())?;
        transaction.commit().map_err(|error| error.to_string())?;
    }

    let commission_migration_applied: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version=4",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if commission_migration_applied == 0 {
        let transaction = connection
            .unchecked_transaction()
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "UPDATE transactions
                 SET commission=CASE type
                       WHEN 'recharge' THEN ROUND(amount*0.10,2)
                       ELSE 0
                     END,
                     commission_rule_version=2
                 WHERE commission_rule_version<2",
                [],
            )
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "UPDATE services
                 SET commission=CASE
                       WHEN package_purchase_id IS NOT NULL THEN commission
                       WHEN service_type='套盒手工' THEN 10
                       ELSE 5
                     END,
                     commission_rule_version=2
                 WHERE commission_rule_version<2",
                [],
            )
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "UPDATE package_purchases SET commission_rule_version=2
                 WHERE commission_rule_version<2",
                [],
            )
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "INSERT INTO schema_migrations(version,applied_at) VALUES (4,?1)",
                params![now],
            )
            .map_err(|error| error.to_string())?;
        transaction.commit().map_err(|error| error.to_string())?;
    }

    let compensation_migration_applied: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version=5",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if compensation_migration_applied == 0 {
        let transaction = connection
            .unchecked_transaction()
            .map_err(|error| error.to_string())?;
        backfill_member_balance_parts(&transaction)?;
        transaction
            .execute(
                "INSERT OR IGNORE INTO employee_compensations
                 (employee_id,base_salary,base_commission_rate,performance_target,
                  excess_commission_rate,meal_allowance_per_day,attendance_bonus,
                  normal_service_commission,package_service_commission,created_at)
                 SELECT id,1800,0.10,10000,0.02,10,300,5,10,?1 FROM employees",
                params![now],
            )
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "INSERT INTO schema_migrations(version,applied_at) VALUES (5,?1)",
                params![now],
            )
            .map_err(|error| error.to_string())?;
        transaction.commit().map_err(|error| error.to_string())?;
    }

    let combined_payment_migration_applied: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version=6",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if combined_payment_migration_applied == 0 {
        if !column_exists(connection, "package_purchases", "balance_payment_amount")? {
            connection
                .execute_batch(
                    "PRAGMA foreign_keys=OFF;
                     BEGIN;
                     ALTER TABLE package_consumptions RENAME TO package_consumptions_legacy_v6;
                     ALTER TABLE package_purchases RENAME TO package_purchases_legacy_v6;
                     CREATE TABLE package_purchases (
                       id TEXT PRIMARY KEY,
                       member_id TEXT NOT NULL REFERENCES members(id),
                       member_name TEXT NOT NULL,
                       employee TEXT NOT NULL,
                       package_id TEXT NOT NULL REFERENCES packages(id),
                       package_name TEXT NOT NULL,
                       price REAL NOT NULL CHECK(price>0),
                       total_uses INTEGER NOT NULL CHECK(total_uses>0),
                       remaining_uses INTEGER NOT NULL CHECK(remaining_uses>=0),
                       payment_method TEXT NOT NULL CHECK(payment_method IN ('会员余额','现金','余额现金组合支付')),
                       balance_payment_amount REAL NOT NULL DEFAULT 0 CHECK(balance_payment_amount>=0),
                       cash_payment_amount REAL NOT NULL DEFAULT 0 CHECK(cash_payment_amount>=0),
                       status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active','completed')),
                       purchased_at TEXT NOT NULL,
                       last_consumed_at TEXT,
                       commission REAL NOT NULL DEFAULT 0,
                       commission_rule_version INTEGER NOT NULL DEFAULT 2,
                       transaction_id TEXT NOT NULL DEFAULT ''
                     );
                     INSERT INTO package_purchases
                       (id,member_id,member_name,employee,package_id,package_name,price,total_uses,
                        remaining_uses,payment_method,balance_payment_amount,cash_payment_amount,
                        status,purchased_at,last_consumed_at,commission,commission_rule_version,
                        transaction_id)
                     SELECT id,member_id,member_name,employee,package_id,package_name,price,total_uses,
                            remaining_uses,payment_method,
                            CASE WHEN payment_method='会员余额' THEN price ELSE 0 END,
                            CASE WHEN payment_method='现金' THEN price ELSE 0 END,
                            status,purchased_at,last_consumed_at,commission,commission_rule_version,
                            transaction_id
                     FROM package_purchases_legacy_v6;
                     CREATE TABLE package_consumptions (
                       id TEXT PRIMARY KEY,
                       package_purchase_id TEXT NOT NULL REFERENCES package_purchases(id),
                       member_id TEXT NOT NULL REFERENCES members(id),
                       member_name TEXT NOT NULL,
                       employee TEXT NOT NULL,
                       package_name TEXT NOT NULL,
                       consumed_at TEXT NOT NULL,
                       remaining_after INTEGER NOT NULL CHECK(remaining_after>=0),
                       commission REAL NOT NULL DEFAULT 0,
                       service_id TEXT NOT NULL REFERENCES services(id),
                       duration INTEGER NOT NULL DEFAULT 60,
                       note TEXT NOT NULL DEFAULT ''
                     );
                     INSERT INTO package_consumptions
                       (id,package_purchase_id,member_id,member_name,employee,package_name,
                        consumed_at,remaining_after,commission,service_id,duration,note)
                     SELECT id,package_purchase_id,member_id,member_name,employee,package_name,
                            consumed_at,remaining_after,commission,service_id,duration,note
                     FROM package_consumptions_legacy_v6;
                     DROP TABLE package_consumptions_legacy_v6;
                     DROP TABLE package_purchases_legacy_v6;
                     COMMIT;
                     PRAGMA foreign_keys=ON;
                     CREATE INDEX IF NOT EXISTS idx_package_purchases_member
                       ON package_purchases(member_id,purchased_at);
                     CREATE INDEX IF NOT EXISTS idx_package_purchases_employee
                       ON package_purchases(employee,purchased_at);
                     CREATE INDEX IF NOT EXISTS idx_package_consumptions_purchase
                       ON package_consumptions(package_purchase_id,consumed_at);",
                )
                .map_err(|error| error.to_string())?;
        }
        connection
            .execute(
                "INSERT INTO schema_migrations(version,applied_at) VALUES (6,?1)",
                params![now],
            )
            .map_err(|error| error.to_string())?;
    }

    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS appointments (
               id TEXT PRIMARY KEY,
               customer_type TEXT NOT NULL CHECK(customer_type IN ('member','guest')),
               member_id TEXT REFERENCES members(id),
               customer_name TEXT NOT NULL,
               customer_phone TEXT NOT NULL,
               employee TEXT NOT NULL,
               service_type TEXT NOT NULL CHECK(service_type IN ('套盒手工','普通手工')),
               service_name TEXT NOT NULL,
               package_purchase_id TEXT REFERENCES package_purchases(id),
               starts_at TEXT NOT NULL,
               duration INTEGER NOT NULL CHECK(duration>0),
               status TEXT NOT NULL DEFAULT 'pending'
                 CHECK(status IN ('pending','arrived','in_service','completed','cancelled','no_show')),
               note TEXT NOT NULL DEFAULT '',
               created_by TEXT NOT NULL,
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL,
               completed_service_id TEXT REFERENCES services(id)
             );
             CREATE INDEX IF NOT EXISTS idx_appointments_starts ON appointments(starts_at);
             CREATE INDEX IF NOT EXISTS idx_appointments_employee ON appointments(employee,starts_at);
             CREATE INDEX IF NOT EXISTS idx_appointments_status ON appointments(status,starts_at);",
        )
        .map_err(|error| error.to_string())?;

    connection
        .execute(
            "INSERT OR IGNORE INTO schema_migrations(version,applied_at) VALUES (7,?1)",
            params![now],
        )
        .map_err(|error| error.to_string())?;

    if column_exists(connection, "members", "level")? {
        connection
            .execute("ALTER TABLE members DROP COLUMN level", [])
            .map_err(|error| error.to_string())?;
    }
    add_column(
        connection,
        "packages",
        "package_type TEXT NOT NULL DEFAULT '套盒' CHECK(package_type IN ('套盒','普通'))",
    )?;
    add_column(
        connection,
        "package_purchases",
        "package_type TEXT NOT NULL DEFAULT '套盒' CHECK(package_type IN ('套盒','普通'))",
    )?;
    let package_model_migration_applied: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version=8",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if package_model_migration_applied == 0 {
        let transaction = connection
            .unchecked_transaction()
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "UPDATE package_purchases
                 SET package_type=COALESCE(
                       (SELECT package_type FROM packages WHERE packages.id=package_purchases.package_id),
                       '套盒'
                     ),
                     commission=ROUND(cash_payment_amount*COALESCE(
                       (SELECT employee_compensations.base_commission_rate
                        FROM employee_compensations
                        JOIN employees ON employees.id=employee_compensations.employee_id
                        WHERE employees.name=package_purchases.employee),
                       0.10
                     ),2),
                     commission_rule_version=3",
                [],
            )
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "INSERT INTO schema_migrations(version,applied_at) VALUES (8,?1)",
                params![now],
            )
            .map_err(|error| error.to_string())?;
        transaction.commit().map_err(|error| error.to_string())?;
    }

    let project_service_migration_applied: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version=9",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if project_service_migration_applied == 0 {
        if !column_exists(connection, "transactions", "source_type")? {
            connection
                .execute_batch(
                    "PRAGMA foreign_keys=OFF;
                     BEGIN;
                     ALTER TABLE transactions RENAME TO transactions_legacy_v9;
                     CREATE TABLE transactions (
                       id TEXT PRIMARY KEY,
                       member_id TEXT REFERENCES members(id),
                       member_name TEXT NOT NULL,
                       type TEXT NOT NULL CHECK(type IN ('recharge','consume')),
                       amount REAL NOT NULL CHECK(amount>0),
                       balance_after REAL NOT NULL,
                       payment_method TEXT NOT NULL,
                       item TEXT NOT NULL,
                       employee TEXT NOT NULL,
                       created_at TEXT NOT NULL,
                       note TEXT NOT NULL DEFAULT '',
                       gift_amount REAL NOT NULL DEFAULT 0,
                       commission REAL NOT NULL DEFAULT 0,
                       commission_rule_version INTEGER NOT NULL DEFAULT 2,
                       status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active','cancelled')),
                       source_type TEXT NOT NULL DEFAULT 'legacy'
                         CHECK(source_type IN ('legacy','service','package_purchase')),
                       source_id TEXT
                     );
                     INSERT INTO transactions
                       (id,member_id,member_name,type,amount,balance_after,payment_method,item,
                        employee,created_at,note,gift_amount,commission,commission_rule_version,
                        status,source_type,source_id)
                     SELECT t.id,t.member_id,t.member_name,t.type,t.amount,t.balance_after,
                            t.payment_method,t.item,t.employee,t.created_at,t.note,t.gift_amount,
                            t.commission,t.commission_rule_version,'active',
                            CASE WHEN EXISTS(
                              SELECT 1 FROM package_purchases p WHERE p.transaction_id=t.id
                            ) THEN 'package_purchase' ELSE 'legacy' END,
                            (SELECT p.id FROM package_purchases p WHERE p.transaction_id=t.id LIMIT 1)
                     FROM transactions_legacy_v9 t;
                     DROP TABLE transactions_legacy_v9;
                     COMMIT;
                     PRAGMA foreign_keys=ON;",
                )
                .map_err(|error| error.to_string())?;
        }
        connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS projects (
                   id TEXT PRIMARY KEY,
                   name TEXT NOT NULL UNIQUE,
                   duration INTEGER NOT NULL CHECK(duration>0),
                   price REAL NOT NULL CHECK(price>0),
                   status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active','inactive')),
                   created_at TEXT NOT NULL,
                   updated_at TEXT NOT NULL
                 );
                 CREATE INDEX IF NOT EXISTS idx_projects_status_name ON projects(status,name);
                 CREATE INDEX IF NOT EXISTS idx_transactions_created ON transactions(created_at);
                 CREATE UNIQUE INDEX IF NOT EXISTS idx_transactions_source
                   ON transactions(source_type,source_id) WHERE source_id IS NOT NULL;",
            )
            .map_err(|error| error.to_string())?;
        add_column(connection, "services", "project_id TEXT")?;
        add_column(
            connection,
            "services",
            "balance_payment_amount REAL NOT NULL DEFAULT 0",
        )?;
        add_column(
            connection,
            "services",
            "external_payment_amount REAL NOT NULL DEFAULT 0",
        )?;
        add_column(
            connection,
            "services",
            "payment_method TEXT NOT NULL DEFAULT ''",
        )?;
        add_column(
            connection,
            "services",
            "gift_deduction REAL NOT NULL DEFAULT 0",
        )?;
        add_column(
            connection,
            "services",
            "principal_deduction REAL NOT NULL DEFAULT 0",
        )?;
        add_column(connection, "services", "transaction_id TEXT")?;
        add_column(connection, "services", "cancelled_at TEXT")?;
        add_column(connection, "appointments", "project_id TEXT")?;
        connection
            .execute_batch(
                "CREATE UNIQUE INDEX IF NOT EXISTS idx_services_transaction
                   ON services(transaction_id) WHERE transaction_id IS NOT NULL;
                 CREATE INDEX IF NOT EXISTS idx_services_project ON services(project_id,created_at);
                 CREATE INDEX IF NOT EXISTS idx_appointments_project ON appointments(project_id);",
            )
            .map_err(|error| error.to_string())?;
        connection
            .execute(
                "INSERT INTO schema_migrations(version,applied_at) VALUES (9,?1)",
                params![now],
            )
            .map_err(|error| error.to_string())?;
    }

    connection
        .execute(
            "UPDATE accounts SET display_name='木子店长' WHERE username='admin' AND display_name='慕姿店长'",
            [],
        )
        .map_err(|error| error.to_string())?;
    connection
        .execute("UPDATE accounts SET must_change_password=0", [])
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub(crate) fn seed_employees(connection: &Connection) -> Result<(), String> {
    let employees = [
        ("e1", "林晓雅", "高级美容师", "#d86c83"),
        ("e2", "周可欣", "美容顾问", "#aa7dce"),
        ("e3", "陈思思", "美容师", "#e7a95f"),
        ("e4", "王小曼", "美容师", "#65a99b"),
    ];
    let now = Utc::now().to_rfc3339();
    for employee in employees {
        connection
            .execute(
                "INSERT OR IGNORE INTO employees (id,name,role,commission_rate,status,color,created_at)
                 VALUES (?1,?2,?3,0,'active',?4,?5)",
                params![employee.0, employee.1, employee.2, employee.3, now],
            )
            .map_err(|error| error.to_string())?;
        connection
            .execute(
                "INSERT INTO employee_status_events(id,employee_id,employee_name,status,effective_at)
                 SELECT ?1,?2,?3,'active',?4 WHERE NOT EXISTS
                 (SELECT 1 FROM employee_status_events WHERE employee_id=?2)",
                params![Uuid::new_v4().to_string(), employee.0, employee.1, now],
            )
            .map_err(|error| error.to_string())?;
        connection
            .execute(
                "INSERT OR IGNORE INTO employee_compensations
                 (employee_id,base_salary,base_commission_rate,performance_target,
                  excess_commission_rate,meal_allowance_per_day,attendance_bonus,
                  normal_service_commission,package_service_commission,created_at)
                 VALUES (?1,1800,0.10,10000,0.02,10,300,5,10,?2)",
                params![employee.0, now],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub(crate) fn seed_default_manager(connection: &Connection) -> Result<(), String> {
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM accounts", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if count == 0 {
        let now = Utc::now().to_rfc3339();
        connection
            .execute(
                "INSERT INTO accounts
                 (id,username,password_hash,display_name,role,employee_id,status,must_change_password,created_at,updated_at)
                 VALUES (?1,'admin',?2,'木子店长','manager',NULL,'active',0,?3,?3)",
                params![Uuid::new_v4().to_string(), hash_password("123456")?, now],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub(crate) fn initialize_database(path: &PathBuf) -> Result<Connection, String> {
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    migrate_database(&connection)?;
    seed_employees(&connection)?;
    seed_default_manager(&connection)?;
    Ok(connection)
}

pub(crate) fn auth_user_from_row(row: &Row<'_>) -> rusqlite::Result<AuthUser> {
    Ok(AuthUser {
        id: row.get(0)?,
        username: row.get(1)?,
        display_name: row.get(2)?,
        role: row.get(3)?,
        employee_id: row.get(4)?,
        status: row.get(5)?,
        must_change_password: row.get::<_, i64>(6)? != 0,
    })
}

pub(crate) fn account_from_row(row: &Row<'_>) -> rusqlite::Result<AccountRecord> {
    Ok(AccountRecord {
        id: row.get(0)?,
        username: row.get(1)?,
        display_name: row.get(2)?,
        role: row.get(3)?,
        employee_id: row.get(4)?,
        status: row.get(5)?,
        must_change_password: row.get::<_, i64>(6)? != 0,
        created_at: row.get(7)?,
    })
}

pub(crate) fn list_accounts(connection: &Connection) -> Result<Vec<AccountRecord>, String> {
    let mut statement = connection
        .prepare(
            "SELECT id,username,display_name,role,employee_id,status,must_change_password,created_at
             FROM accounts ORDER BY role,created_at",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], account_from_row)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(rows)
}

pub(crate) fn snapshot(connection: &Connection) -> Result<AppSnapshot, String> {
    let members = {
        let mut statement = connection.prepare("SELECT id,name,phone,balance,principal_balance,gift_balance,total_recharge,total_consumption,join_date,last_visit,status FROM members ORDER BY join_date DESC").map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok(Member {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    phone: row.get(2)?,
                    balance: row.get(3)?,
                    principal_balance: row.get(4)?,
                    gift_balance: row.get(5)?,
                    total_recharge: row.get(6)?,
                    total_consumption: row.get(7)?,
                    join_date: row.get(8)?,
                    last_visit: row.get(9)?,
                    status: row.get(10)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };
    let transactions = {
        let mut statement = connection.prepare("SELECT id,member_id,member_name,type,amount,gift_amount,balance_after,payment_method,item,employee,commission,commission_rule_version,created_at,note,status,source_type,source_id FROM transactions ORDER BY created_at DESC").map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok(TransactionRecord {
                    id: row.get(0)?,
                    member_id: row.get(1)?,
                    member_name: row.get(2)?,
                    r#type: row.get(3)?,
                    amount: row.get(4)?,
                    gift_amount: row.get(5)?,
                    balance_after: row.get(6)?,
                    payment_method: row.get(7)?,
                    item: row.get(8)?,
                    employee: row.get(9)?,
                    commission: row.get(10)?,
                    commission_rule_version: row.get(11)?,
                    created_at: row.get(12)?,
                    note: row.get(13)?,
                    status: row.get(14)?,
                    source_type: row.get(15)?,
                    source_id: row.get(16)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };
    let services = {
        let mut statement = connection.prepare("SELECT id,member_id,member_name,employee,service_name,service_type,duration,amount,commission,commission_rule_version,package_purchase_id,project_id,balance_payment_amount,external_payment_amount,payment_method,gift_deduction,principal_deduction,transaction_id,created_at,status FROM services ORDER BY created_at DESC").map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok(ServiceRecord {
                    id: row.get(0)?,
                    member_id: row.get(1)?,
                    member_name: row.get(2)?,
                    employee: row.get(3)?,
                    service_name: row.get(4)?,
                    service_type: row.get(5)?,
                    duration: row.get(6)?,
                    amount: row.get(7)?,
                    commission: row.get(8)?,
                    commission_rule_version: row.get(9)?,
                    package_purchase_id: row.get(10)?,
                    project_id: row.get(11)?,
                    balance_payment_amount: row.get(12)?,
                    external_payment_amount: row.get(13)?,
                    payment_method: row.get(14)?,
                    gift_deduction: row.get(15)?,
                    principal_deduction: row.get(16)?,
                    transaction_id: row.get(17)?,
                    created_at: row.get(18)?,
                    status: row.get(19)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };
    let projects = {
        let mut statement = connection
            .prepare(
                "SELECT id,name,duration,price,status,created_at,updated_at
                 FROM projects ORDER BY created_at DESC,id",
            )
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok(ProjectDefinition {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    duration: row.get(2)?,
                    price: row.get(3)?,
                    status: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };
    let employees = {
        let mut statement = connection
            .prepare(
                "SELECT id,name,role,status,color,created_at FROM employees ORDER BY created_at,id",
            )
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok(Employee {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    role: row.get(2)?,
                    status: row.get(3)?,
                    color: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };
    let employee_compensations = {
        let mut statement = connection
            .prepare(
                "SELECT employee_id,base_salary,base_commission_rate,performance_target,
                    excess_commission_rate,meal_allowance_per_day,attendance_bonus,
                    normal_service_commission,package_service_commission,created_at
             FROM employee_compensations ORDER BY employee_id",
            )
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok(EmployeeCompensation {
                    employee_id: row.get(0)?,
                    base_salary: row.get(1)?,
                    base_commission_rate: row.get(2)?,
                    performance_target: row.get(3)?,
                    excess_commission_rate: row.get(4)?,
                    meal_allowance_per_day: row.get(5)?,
                    attendance_bonus: row.get(6)?,
                    normal_service_commission: row.get(7)?,
                    package_service_commission: row.get(8)?,
                    created_at: row.get(9)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };
    let employee_status_events = {
        let mut statement = connection.prepare("SELECT id,employee_id,employee_name,status,effective_at FROM employee_status_events ORDER BY effective_at").map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok(EmployeeStatusEvent {
                    id: row.get(0)?,
                    employee_id: row.get(1)?,
                    status: row.get(3)?,
                    changed_at: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };
    let commission_configs = {
        let mut statement = connection.prepare("SELECT id,effective_month,recharge_rate,package_purchase_rate,package_service_commission,normal_service_commission,base_salary,created_at FROM commission_configs ORDER BY effective_month").map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok(CommissionConfig {
                    id: row.get(0)?,
                    effective_month: row.get(1)?,
                    recharge_rate: row.get(2)?,
                    package_purchase_rate: row.get(3)?,
                    package_service_amount: row.get(4)?,
                    normal_service_amount: row.get(5)?,
                    base_salary: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };
    let attendance_records = {
        let mut statement = connection.prepare("SELECT id,employee_id,employee_name,month,rest_days,created_at,updated_at FROM attendance_records ORDER BY month DESC,employee_name").map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok(AttendanceRecord {
                    id: row.get(0)?,
                    employee_id: row.get(1)?,
                    month: row.get(3)?,
                    rest_days: row.get(4)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };
    let packages = {
        let mut statement = connection.prepare("SELECT id,name,price,total_uses,package_type,status,created_at,updated_at FROM packages ORDER BY created_at DESC").map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok(PackageDefinition {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    price: row.get(2)?,
                    total_uses: row.get(3)?,
                    package_type: row.get(4)?,
                    status: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };
    let package_purchases = {
        let mut statement = connection.prepare("SELECT id,member_id,member_name,employee,package_id,package_name,package_type,price,total_uses,remaining_uses,payment_method,balance_payment_amount,cash_payment_amount,status,purchased_at,last_consumed_at,commission,commission_rule_version,transaction_id FROM package_purchases ORDER BY purchased_at DESC").map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok(PackagePurchase {
                    id: row.get(0)?,
                    member_id: row.get(1)?,
                    member_name: row.get(2)?,
                    employee: row.get(3)?,
                    package_id: row.get(4)?,
                    package_name: row.get(5)?,
                    package_type: row.get(6)?,
                    price: row.get(7)?,
                    total_uses: row.get(8)?,
                    remaining_uses: row.get(9)?,
                    payment_method: row.get(10)?,
                    balance_payment_amount: row.get(11)?,
                    cash_payment_amount: row.get(12)?,
                    status: row.get(13)?,
                    purchased_at: row.get(14)?,
                    last_consumed_at: row.get(15)?,
                    commission: row.get(16)?,
                    commission_rule_version: row.get(17)?,
                    transaction_id: row.get(18)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };
    let package_consumptions = {
        let mut statement = connection.prepare("SELECT id,package_purchase_id,employee,duration,remaining_after,consumed_at,note,service_id FROM package_consumptions ORDER BY consumed_at DESC").map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok(PackageConsumption {
                    id: row.get(0)?,
                    purchase_id: row.get(1)?,
                    employee: row.get(2)?,
                    duration: row.get(3)?,
                    remaining_after: row.get(4)?,
                    created_at: row.get(5)?,
                    note: row.get(6)?,
                    service_id: row.get(7)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };
    let appointments = {
        let mut statement = connection
            .prepare(
                "SELECT id,customer_type,member_id,customer_name,customer_phone,employee,
                    service_type,service_name,project_id,package_purchase_id,starts_at,duration,status,note,
                    created_by,created_at,updated_at,completed_service_id
             FROM appointments ORDER BY starts_at DESC,id",
            )
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok(Appointment {
                    id: row.get(0)?,
                    customer_type: row.get(1)?,
                    member_id: row.get(2)?,
                    customer_name: row.get(3)?,
                    customer_phone: row.get(4)?,
                    employee: row.get(5)?,
                    service_type: row.get(6)?,
                    service_name: row.get(7)?,
                    project_id: row.get(8)?,
                    package_purchase_id: row.get(9)?,
                    starts_at: row.get(10)?,
                    duration: row.get(11)?,
                    status: row.get(12)?,
                    note: row.get(13)?,
                    created_by: row.get(14)?,
                    created_at: row.get(15)?,
                    updated_at: row.get(16)?,
                    completed_service_id: row.get(17)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };

    Ok(AppSnapshot {
        members,
        transactions,
        services,
        projects,
        employees,
        employee_compensations,
        employee_status_events,
        commission_configs,
        attendance_records,
        packages,
        package_purchases,
        package_consumptions,
        appointments,
    })
}

pub(crate) fn snapshot_for_user(
    connection: &Connection,
    user: &AuthUser,
) -> Result<AppSnapshot, String> {
    let mut data = snapshot(connection)?;
    if user.role != "manager" {
        data.employee_compensations.clear();
        data.employee_status_events.clear();
        data.commission_configs.clear();
        data.attendance_records.clear();
        data.transactions
            .iter_mut()
            .for_each(|item| item.commission = 0.0);
        data.services
            .iter_mut()
            .for_each(|item| item.commission = 0.0);
        data.package_purchases
            .iter_mut()
            .for_each(|item| item.commission = 0.0);
    }
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_balance_migration_replays_gifts_and_consumption_in_order() {
        let records = vec![
            LegacyBalanceEvent {
                record_type: "recharge".to_string(),
                amount: 1000.0,
                gift_amount: 200.0,
                balance_after: 1200.0,
                payment_method: "现金".to_string(),
                item: "会员充值".to_string(),
            },
            LegacyBalanceEvent {
                record_type: "consume".to_string(),
                amount: 150.0,
                gift_amount: 0.0,
                balance_after: 1050.0,
                payment_method: "会员余额".to_string(),
                item: "普通项目".to_string(),
            },
        ];
        assert_eq!(
            split_legacy_member_balance(1050.0, &records),
            (1000.0, 50.0)
        );
    }

    #[test]
    fn migration_is_idempotent_on_a_new_database() {
        let connection = Connection::open_in_memory().expect("open database");
        migrate_database(&connection).expect("first migration");
        seed_employees(&connection).expect("seed employees");
        migrate_database(&connection).expect("second migration");
        let data = snapshot(&connection).expect("snapshot");
        assert_eq!(data.employees.len(), 4);
        assert!(!data.commission_configs.is_empty());
        assert_eq!(data.employee_status_events.len(), 4);
        assert_eq!(data.employee_compensations.len(), 4);
        assert!(!column_exists(&connection, "members", "level").expect("member schema"));
    }

    #[test]
    fn migration_preserves_package_records_and_backfills_payment_parts() {
        let connection = Connection::open_in_memory().expect("open database");
        migrate_database(&connection).expect("initialize current database");
        connection
            .execute_batch(
                "PRAGMA foreign_keys=OFF;
                 DROP TABLE package_consumptions;
                 DROP TABLE package_purchases;
                 DELETE FROM schema_migrations WHERE version IN (6,8);
                 CREATE TABLE package_purchases (
                   id TEXT PRIMARY KEY,member_id TEXT NOT NULL,member_name TEXT NOT NULL,
                   employee TEXT NOT NULL,package_id TEXT NOT NULL,package_name TEXT NOT NULL,
                   price REAL NOT NULL,total_uses INTEGER NOT NULL,remaining_uses INTEGER NOT NULL,
                   payment_method TEXT NOT NULL CHECK(payment_method IN ('会员余额','现金')),
                   status TEXT NOT NULL,purchased_at TEXT NOT NULL,last_consumed_at TEXT,
                   commission REAL NOT NULL DEFAULT 0,commission_rule_version INTEGER NOT NULL DEFAULT 2,
                   transaction_id TEXT NOT NULL DEFAULT ''
                 );
                 CREATE TABLE package_consumptions (
                   id TEXT PRIMARY KEY,package_purchase_id TEXT NOT NULL,member_id TEXT NOT NULL,
                   member_name TEXT NOT NULL,employee TEXT NOT NULL,package_name TEXT NOT NULL,
                   consumed_at TEXT NOT NULL,remaining_after INTEGER NOT NULL,commission REAL NOT NULL,
                   service_id TEXT NOT NULL,duration INTEGER NOT NULL DEFAULT 60,note TEXT NOT NULL DEFAULT ''
                 );
                 INSERT INTO members
                   (id,name,phone,balance,principal_balance,gift_balance,total_recharge,
                    total_consumption,join_date,last_visit,status)
                 VALUES ('m','会员','138',200,200,0,1000,800,
                         '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z','active');
                 INSERT INTO packages
                   (id,name,price,total_uses,package_type,status,created_at,updated_at)
                 VALUES ('p','护理套盒',800,10,'套盒','active','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z');
                 INSERT INTO services
                   (id,member_id,member_name,employee,service_name,service_type,duration,amount,
                    commission,commission_rule_version,created_at,status,package_purchase_id)
                 VALUES ('s','m','会员','员工','护理套盒','套盒手工',60,0,10,2,
                         '2026-01-02T00:00:00Z','completed','pp');
                 INSERT INTO package_purchases
                 VALUES ('pp','m','会员','员工','p','护理套盒',800,10,9,'会员余额','active',
                         '2026-01-01T00:00:00Z','2026-01-02T00:00:00Z',80,2,'t');
                 INSERT INTO package_consumptions
                 VALUES ('pc','pp','m','会员','员工','护理套盒','2026-01-02T00:00:00Z',9,10,'s',60,'');
                 PRAGMA foreign_keys=ON;",
            )
            .expect("prepare previous package schema");

        migrate_database(&connection).expect("migrate package payments");
        let data = snapshot(&connection).expect("snapshot");
        assert_eq!(data.package_purchases.len(), 1);
        assert_eq!(data.package_purchases[0].balance_payment_amount, 800.0);
        assert_eq!(data.package_purchases[0].cash_payment_amount, 0.0);
        assert_eq!(data.package_purchases[0].package_type, "套盒");
        assert_eq!(data.package_purchases[0].commission, 0.0);
        assert_eq!(data.package_purchases[0].commission_rule_version, 3);
        assert_eq!(data.package_consumptions.len(), 1);
        let foreign_key_errors: i64 = connection
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .expect("foreign key check");
        assert_eq!(foreign_key_errors, 0);
    }

    #[test]
    fn migration_preserves_legacy_records_and_backfills_commission() {
        let connection = Connection::open_in_memory().expect("open database");
        connection
            .execute_batch(
                "PRAGMA foreign_keys=ON;
                 CREATE TABLE employees(id TEXT PRIMARY KEY,name TEXT UNIQUE,role TEXT,commission_rate REAL,status TEXT,color TEXT);
                 CREATE TABLE members(id TEXT PRIMARY KEY,name TEXT,phone TEXT,level TEXT,balance REAL,total_recharge REAL,total_consumption REAL,join_date TEXT,last_visit TEXT,status TEXT);
                 CREATE TABLE transactions(id TEXT PRIMARY KEY,member_id TEXT NOT NULL REFERENCES members(id),member_name TEXT,type TEXT,amount REAL,balance_after REAL,payment_method TEXT,item TEXT,employee TEXT,created_at TEXT,note TEXT,commission REAL);
                 CREATE TABLE services(id TEXT PRIMARY KEY,member_id TEXT NOT NULL REFERENCES members(id),member_name TEXT,employee TEXT,service_name TEXT,duration INTEGER,amount REAL,commission REAL,created_at TEXT,status TEXT);
                 CREATE TABLE accounts(id TEXT PRIMARY KEY,username TEXT UNIQUE,password_hash TEXT,display_name TEXT,role TEXT,employee_id TEXT,status TEXT,must_change_password INTEGER,created_at TEXT,updated_at TEXT);
                 INSERT INTO employees VALUES('e','员工','美容师',0,'active','#000');
                 INSERT INTO members VALUES('m','会员','138','普通会员',100,100,0,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z','active');
                 INSERT INTO transactions VALUES('t','m','会员','recharge',100,100,'现金','充值','员工','2026-01-01T00:00:00Z','',654.12);
                 INSERT INTO services VALUES('s','m','会员','员工','护理',60,100,654.12,'2026-01-01T00:00:00Z','completed');",
            )
            .expect("legacy schema");
        migrate_database(&connection).expect("migrate legacy database");
        let data = snapshot(&connection).expect("snapshot");
        assert_eq!(data.transactions[0].commission, 10.0);
        assert_eq!(data.transactions[0].commission_rule_version, 2);
        assert_eq!(data.services[0].commission, 5.0);
        assert_eq!(data.services[0].commission_rule_version, 2);
        assert!(data.services[0].member_id.is_some());
        assert_eq!(data.employees[0].created_at, "2026-01-01T00:00:00Z");
        assert_eq!(data.members[0].principal_balance, 100.0);
        assert_eq!(data.members[0].gift_balance, 0.0);
        assert_eq!(data.employee_compensations[0].base_salary, 1800.0);
        assert_eq!(data.employee_compensations[0].performance_target, 10000.0);

        connection
            .execute(
                "UPDATE services SET commission=37.25
                 WHERE id='s' AND commission_rule_version=2",
                [],
            )
            .expect("update current commission snapshot");
        migrate_database(&connection).expect("repeat migration");
        let migrated_again = snapshot(&connection).expect("snapshot after repeat migration");
        assert_eq!(migrated_again.services[0].commission, 37.25);
        assert_eq!(migrated_again.services[0].commission_rule_version, 2);

        let employee_snapshot = snapshot_for_user(
            &connection,
            &AuthUser {
                id: "account".to_string(),
                username: "employee".to_string(),
                display_name: "员工账号".to_string(),
                role: "employee".to_string(),
                employee_id: None,
                status: "active".to_string(),
                must_change_password: false,
            },
        )
        .expect("employee snapshot");
        assert!(employee_snapshot.commission_configs.is_empty());
        assert!(employee_snapshot.employee_compensations.is_empty());
        assert!(employee_snapshot.attendance_records.is_empty());
        assert_eq!(employee_snapshot.transactions[0].commission, 0.0);
        assert_eq!(employee_snapshot.services[0].commission, 0.0);
        let foreign_key_errors: i64 = connection
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .expect("foreign key check");
        assert_eq!(foreign_key_errors, 0);
    }
}
