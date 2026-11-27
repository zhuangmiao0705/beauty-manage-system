use crate::{database::raw_snapshot, models::*};
use chrono::{DateTime, Local};
use rusqlite::{params, Connection};
use std::collections::HashMap;

fn round_money(value: f64) -> f64 {
    let cents = (value.abs() * 100.0 + 0.5 + 1e-8).floor();
    if cents == 0.0 {
        0.0
    } else {
        value.signum() * cents / 100.0
    }
}

pub(crate) fn validate(mode: &str, rate: f64) -> Result<(), String> {
    if !["general", "independent"].contains(&mode)
        || !rate.is_finite()
        || !(0.0..=1.0).contains(&rate)
    {
        return Err("请选择有效的提成方式，独立提成比例必须在0%至100%之间".into());
    }
    Ok(())
}

#[derive(Clone)]
struct Lot {
    allocation: PrincipalAllocation,
    remaining: f64,
    remaining_commission: f64,
}

fn take(wallet: &mut Vec<Lot>, amount: f64) -> Vec<PrincipalAllocation> {
    let mut remaining = round_money(amount);
    let mut result = Vec::new();
    for lot in wallet.iter_mut() {
        if remaining <= 0.0 {
            break;
        }
        let used = round_money(remaining.min(lot.remaining));
        if used <= 0.0 {
            continue;
        }
        let credit = if used == lot.remaining {
            lot.remaining_commission
        } else {
            round_money(lot.allocation.commission * used / lot.allocation.amount)
                .min(lot.remaining_commission)
        };
        result.push(PrincipalAllocation {
            recharge_id: lot.allocation.recharge_id.clone(),
            amount: used,
            commission: credit,
            verified: lot.allocation.verified,
        });
        lot.remaining = round_money(lot.remaining - used);
        lot.remaining_commission = round_money(lot.remaining_commission - credit);
        remaining = round_money(remaining - used);
    }
    if remaining > 0.0 {
        result.push(PrincipalAllocation {
            recharge_id: String::new(),
            amount: remaining,
            commission: 0.0,
            verified: false,
        });
    }
    result
}

fn take_recorded(
    wallet: &mut Vec<Lot>,
    amount: f64,
    existing: Option<&CommissionEntry>,
) -> Vec<PrincipalAllocation> {
    let Some(entry) = existing else {
        return take(wallet, amount);
    };
    for allocation in &entry.principal_allocations {
        let mut remaining = allocation.amount;
        let mut credit = allocation.commission;
        for lot in wallet
            .iter_mut()
            .filter(|item| item.allocation.recharge_id == allocation.recharge_id)
        {
            let used = round_money(remaining.min(lot.remaining));
            let used_credit = if used == remaining {
                credit
            } else {
                round_money(used * allocation.commission / allocation.amount).min(credit)
            };
            lot.remaining = round_money(lot.remaining - used);
            lot.remaining_commission =
                round_money((lot.remaining_commission - used_credit).max(0.0));
            remaining = round_money(remaining - used);
            credit = round_money(credit - used_credit);
            if remaining <= 0.0 {
                break;
            }
        }
    }
    entry.principal_allocations.clone()
}

fn give_back(wallet: &mut Vec<Lot>, allocations: &[PrincipalAllocation], amount: f64) {
    let total = round_money(allocations.iter().map(|item| item.amount).sum());
    let mut remaining = round_money(total.min(amount));
    for (index, item) in allocations.iter().enumerate() {
        let restored = if index == allocations.len() - 1 {
            remaining
        } else {
            round_money(amount * item.amount / total).min(remaining)
        };
        remaining = round_money(remaining - restored);
        if restored <= 0.0 {
            continue;
        }
        let credit = if restored == item.amount {
            item.commission
        } else {
            round_money(item.commission * restored / item.amount)
        };
        if let Some(original) = wallet
            .iter_mut()
            .find(|lot| lot.allocation.recharge_id == item.recharge_id)
        {
            original.remaining = round_money(original.remaining + restored);
            original.remaining_commission = round_money(original.remaining_commission + credit);
            continue;
        }
        wallet.push(Lot {
            allocation: PrincipalAllocation {
                recharge_id: item.recharge_id.clone(),
                amount: restored,
                commission: credit,
                verified: item.verified,
            },
            remaining: restored,
            remaining_commission: credit,
        });
    }
}

struct Event {
    kind: &'static str,
    id: String,
    time: String,
    priority: i32,
    source: serde_json::Value,
}
fn text(value: &serde_json::Value, key: &str) -> String {
    value[key].as_str().unwrap_or_default().to_string()
}
fn number(value: &serde_json::Value, key: &str) -> f64 {
    value[key].as_f64().unwrap_or(0.0)
}
fn month(value: &str) -> String {
    DateTime::parse_from_rfc3339(value)
        .map(|date| date.with_timezone(&Local).format("%Y-%m").to_string())
        .unwrap_or_else(|_| value.chars().take(7).collect())
}

pub(crate) fn synchronize(
    snapshot: &AppSnapshot,
    legacy: bool,
) -> Result<Vec<CommissionEntry>, String> {
    let mut saved: HashMap<String, CommissionEntry> = snapshot
        .commission_entries
        .iter()
        .map(|entry| (entry.id.clone(), entry.clone()))
        .collect();
    let mut processed: Vec<CommissionEntry> = Vec::new();
    let mut wallets: HashMap<String, Vec<Lot>> = HashMap::new();
    let mut gifts: HashMap<String, f64> = HashMap::new();
    let mut events: Vec<Event> = Vec::new();
    let mut add = |kind, id: &str, time: &str, priority, source| {
        events.push(Event {
            kind,
            id: id.into(),
            time: time.into(),
            priority,
            source,
        });
    };
    for item in &snapshot.transactions {
        if item.r#type == "recharge" {
            add(
                "recharge",
                &item.id,
                &item.created_at,
                0,
                serde_json::json!(item),
            );
        } else if item.status != "cancelled"
            && !snapshot
                .services
                .iter()
                .any(|service| service.transaction_id.as_deref() == Some(&item.id))
            && !snapshot
                .package_purchases
                .iter()
                .any(|purchase| purchase.transaction_id == item.id)
        {
            add(
                "legacy",
                &item.id,
                &item.created_at,
                2,
                serde_json::json!(item),
            );
        }
    }
    for item in &snapshot.package_purchases {
        add(
            "package",
            &item.id,
            &item.purchased_at,
            1,
            serde_json::json!(item),
        );
    }
    for item in &snapshot.services {
        if item.package_purchase_id.is_some() {
            continue;
        }
        add(
            "service",
            &item.id,
            &item.created_at,
            2,
            serde_json::json!(item),
        );
        if item.status == "cancelled" {
            add(
                "cancel",
                &item.id,
                item.cancelled_at.as_deref().unwrap_or(&item.created_at),
                4,
                serde_json::json!(item),
            );
        }
    }
    for item in &snapshot.refund_records {
        add(
            "refund",
            &item.id,
            &item.created_at,
            3,
            serde_json::json!(item),
        );
    }
    events.sort_by(|a, b| {
        DateTime::parse_from_rfc3339(&a.time)
            .ok()
            .cmp(&DateTime::parse_from_rfc3339(&b.time).ok())
            .then(a.priority.cmp(&b.priority))
            .then(a.id.cmp(&b.id))
    });
    for event in events {
        let key = format!("{}:{}", event.kind, event.id);
        let source = &event.source;
        let member_id = source["memberId"].as_str().map(str::to_string);
        let member_key = member_id.clone().unwrap_or_default();
        let amount;
        let mut balance = 0.0;
        let mut cash = 0.0;
        let mut performance = 0.0;
        let mut mode = "general".to_string();
        let mut rate = 0.0;
        let mut principal = Vec::new();
        let mut names = Vec::new();
        let name: String;
        let mut original: Option<CommissionEntry> = None;
        match event.kind {
            "recharge" => {
                amount = number(source, "amount");
                cash = amount;
                performance = amount;
                names.push(text(source, "employee"));
                name = text(source, "item");
                let gift = gifts.entry(member_key.clone()).or_default();
                *gift = round_money(*gift + number(source, "giftAmount"));
            }
            "package" => {
                amount = number(source, "price");
                balance = number(source, "balancePaymentAmount");
                cash = number(source, "cashPaymentAmount");
                mode = text(source, "commissionMode");
                rate = number(source, "commissionRate");
                performance = if mode == "general" { cash } else { 0.0 };
                names.push(text(source, "employee"));
                name = text(source, "packageName");
                principal = take_recorded(
                    wallets.entry(member_key.clone()).or_default(),
                    balance,
                    saved.get(&key),
                );
            }
            "service" => {
                amount = number(source, "amount");
                balance = number(source, "principalDeduction");
                cash = number(source, "externalPaymentAmount");
                mode = text(source, "commissionMode");
                rate = number(source, "commissionRate");
                name = text(source, "serviceName");
                names = source["employees"]
                    .as_array()
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(|item| item.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                if names.is_empty() {
                    names.push(text(source, "employee"));
                }
                principal = take_recorded(
                    wallets.entry(member_key.clone()).or_default(),
                    balance,
                    saved.get(&key),
                );
                let gift = gifts.entry(member_key.clone()).or_default();
                *gift = round_money((*gift - number(source, "giftDeduction")).max(0.0));
            }
            "legacy" => {
                amount = number(source, "amount");
                name = text(source, "item");
                if text(source, "paymentMethod").contains("会员余额") {
                    let gift = gifts.entry(member_key.clone()).or_default();
                    let is_package =
                        name.starts_with("套盒购买：") || name.starts_with("套餐购买：");
                    let gift_deduction = if is_package { 0.0 } else { gift.min(amount) };
                    balance = saved
                        .get(&key)
                        .map(|entry| entry.balance_amount)
                        .unwrap_or_else(|| round_money(amount - gift_deduction));
                    principal = take_recorded(
                        wallets.entry(member_key.clone()).or_default(),
                        balance,
                        saved.get(&key),
                    );
                    *gift = round_money(*gift - gift_deduction);
                }
            }
            "cancel" => {
                original = saved.get(&format!("service:{}", event.id)).cloned();
                if let Some(ref original) = original {
                    give_back(
                        wallets.entry(member_key.clone()).or_default(),
                        &original.principal_allocations,
                        original.balance_amount,
                    );
                    mode = original.mode.clone();
                    rate = original.rate;
                    balance = -original.balance_amount;
                    cash = -original.cash_amount;
                }
                let gift = gifts.entry(member_key.clone()).or_default();
                *gift = round_money(*gift + number(source, "giftDeduction"));
                amount = -number(source, "amount");
                name = format!("撤销：{}", text(source, "serviceName"));
            }
            _ => {
                amount = -number(source, "amount");
                names.push(text(source, "employee"));
                if text(source, "refundType") == "account" {
                    name = "会员账户退款".into();
                    performance = -number(source, "cashAmount");
                    cash = performance;
                    let gift = gifts.entry(member_key.clone()).or_default();
                    balance = -cash + (number(source, "giftForfeitedAmount") - *gift).max(0.0);
                    principal = take_recorded(
                        wallets.entry(member_key.clone()).or_default(),
                        balance,
                        saved.get(&key),
                    );
                    *gift = round_money((*gift - number(source, "giftForfeitedAmount")).max(0.0));
                } else {
                    name = "套餐退款".into();
                    original = saved
                        .get(&format!("package:{}", text(source, "packagePurchaseId")))
                        .cloned();
                    if let Some(ref original) = original {
                        give_back(
                            wallets.entry(member_key.clone()).or_default(),
                            &original.principal_allocations,
                            number(source, "balanceAmount"),
                        );
                        mode = original.mode.clone();
                        rate = original.rate;
                    }
                    balance = -number(source, "balanceAmount");
                    cash = -number(source, "cashAmount");
                    performance = if mode == "general" { cash } else { 0.0 };
                }
            }
        }
        let entry = if let Some(entry) = saved.get(&key) {
            entry.clone()
        } else {
            if !legacy
                && mode == "independent"
                && principal
                    .iter()
                    .any(|item| !item.verified && item.amount > 0.0)
            {
                return Err("账户本金的历史充值来源不完整，无法确认已计提成。请核对充值记录或改用额外付款。".into());
            }
            let mut commission = 0.0;
            let mut offset = 0.0;
            let mut balance_commission = 0.0;
            let mut cash_commission = 0.0;
            let mut employees = Vec::new();
            let mut employee_ids = Vec::new();
            for employee_name in names {
                if let Some(employee) = snapshot
                    .employees
                    .iter()
                    .find(|item| item.name == employee_name)
                {
                    if !employee_ids.contains(&employee.id) {
                        employee_ids.push(employee.id.clone());
                    }
                }
            }
            if mode == "independent" && ["package", "service"].contains(&event.kind) {
                offset = round_money(principal.iter().map(|item| item.commission).sum());
                balance_commission = round_money(
                    principal
                        .iter()
                        .map(|item| (round_money(item.amount * rate) - item.commission).max(0.0))
                        .sum(),
                );
                cash_commission = round_money(cash * rate);
                commission = round_money(balance_commission + cash_commission);
            } else if mode == "independent" {
                if let Some(ref original) = original {
                    let ratio = if event.kind == "cancel" {
                        1.0
                    } else if original.amount > 0.0 {
                        -amount / original.amount
                    } else {
                        0.0
                    };
                    commission = -round_money(original.commission * ratio);
                    offset = -round_money(original.recharge_offset * ratio);
                    balance_commission = -round_money(original.balance_commission * ratio);
                    cash_commission = round_money(commission - balance_commission);
                    let mut remaining = commission;
                    employees = original
                        .employees
                        .iter()
                        .enumerate()
                        .map(|(index, item)| {
                            let share = if index == original.employees.len() - 1 {
                                remaining
                            } else {
                                -round_money(item.commission * ratio)
                            };
                            remaining = round_money(remaining - share);
                            CommissionEmployee {
                                employee_id: item.employee_id.clone(),
                                commission: share,
                            }
                        })
                        .collect();
                }
            } else if performance != 0.0 && !employee_ids.is_empty() {
                let employee_id = &employee_ids[0];
                let previous = round_money(
                    processed
                        .iter()
                        .filter(|item| {
                            item.mode == "general"
                                && item
                                    .employees
                                    .iter()
                                    .any(|employee| &employee.employee_id == employee_id)
                                && month(&item.created_at) == month(&event.time)
                        })
                        .map(|item| item.performance_amount)
                        .sum(),
                );
                let compensation = snapshot
                    .employee_compensations
                    .iter()
                    .find(|item| &item.employee_id == employee_id);
                let (base, target, excess) = compensation
                    .map(|item| {
                        (
                            item.base_commission_rate,
                            item.performance_target,
                            item.excess_commission_rate,
                        )
                    })
                    .unwrap_or((0.1, 10000.0, 0.02));
                let cumulative =
                    |value: f64| round_money(value * base + (value - target).max(0.0) * excess);
                commission = round_money(cumulative(previous + performance) - cumulative(previous));
                rate = commission / performance;
                cash_commission = commission;
            }
            if employees.is_empty() {
                let mut remaining = commission;
                for (index, employee_id) in employee_ids.iter().enumerate() {
                    let share = if index == employee_ids.len() - 1 {
                        remaining
                    } else {
                        round_money(commission / employee_ids.len() as f64)
                    };
                    remaining = round_money(remaining - share);
                    employees.push(CommissionEmployee {
                        employee_id: employee_id.clone(),
                        commission: share,
                    });
                }
            }
            let entry = CommissionEntry {
                id: key.clone(),
                source_type: event.kind.into(),
                source_id: event.id.clone(),
                member_id: member_id.clone(),
                member_name: text(source, "memberName"),
                business_name: name,
                mode,
                rate,
                amount,
                balance_amount: balance,
                cash_amount: cash,
                performance_amount: performance,
                recharge_offset: offset,
                balance_commission,
                cash_commission,
                commission,
                principal_allocations: principal,
                employees,
                created_at: event.time,
                legacy,
            };
            saved.insert(key, entry.clone());
            entry
        };
        if event.kind == "recharge" {
            wallets.entry(member_key).or_default().push(Lot {
                allocation: PrincipalAllocation {
                    recharge_id: event.id,
                    amount,
                    commission: entry.commission,
                    verified: true,
                },
                remaining: amount,
                remaining_commission: entry.commission,
            });
        }
        processed.push(entry);
    }
    Ok(processed)
}

pub(crate) fn sync(connection: &Connection, legacy: bool) -> Result<(), String> {
    let pending: bool = connection
        .query_row(
            "SELECT
        (SELECT COUNT(*) FROM commission_entries) <>
        ((SELECT COUNT(*) FROM transactions WHERE type='recharge') +
         (SELECT COUNT(*) FROM package_purchases) +
         (SELECT COUNT(*) FROM services WHERE package_purchase_id IS NULL) +
         (SELECT COUNT(*) FROM services WHERE package_purchase_id IS NULL AND status='cancelled') +
         (SELECT COUNT(*) FROM refund_records) +
         (SELECT COUNT(*) FROM transactions t WHERE type<>'recharge' AND status<>'cancelled'
           AND NOT EXISTS(SELECT 1 FROM services s WHERE s.transaction_id=t.id)
           AND NOT EXISTS(SELECT 1 FROM package_purchases p WHERE p.transaction_id=t.id)))",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())?;
    if !pending {
        return Ok(());
    }
    let data = raw_snapshot(connection)?;
    let entries = synchronize(&data, legacy)?;
    for entry in entries {
        if data
            .commission_entries
            .iter()
            .any(|item| item.id == entry.id)
        {
            continue;
        }
        connection
            .execute(
                "INSERT INTO commission_entries(id,payload) VALUES (?1,?2)",
                params![
                    entry.id,
                    serde_json::to_string(&entry).map_err(|error| error.to_string())?
                ],
            )
            .map_err(|error| error.to_string())?;
        let table = match entry.source_type.as_str() {
            "recharge" => Some("transactions"),
            "package" => Some("package_purchases"),
            "refund" => Some("refund_records"),
            "service" if entry.mode == "independent" => Some("services"),
            _ => None,
        };
        if let Some(table) = table {
            connection
                .execute(
                    &format!("UPDATE {table} SET commission=?1 WHERE id=?2"),
                    params![
                        if entry.source_type == "refund" {
                            -entry.commission
                        } else {
                            entry.commission
                        },
                        entry.source_id
                    ],
                )
                .map_err(|error| error.to_string())?;
        }
        if entry.source_type == "service" && entry.mode == "independent" {
            for employee in &entry.employees {
                connection
                    .execute(
                        "UPDATE service_employees SET commission=?1 WHERE service_id=?2
                    AND employee=(SELECT name FROM employees WHERE id=?3)",
                        params![employee.commission, entry.source_id, employee.employee_id],
                    )
                    .map_err(|error| error.to_string())?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{migrate_database, snapshot};

    fn setup() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        migrate_database(&connection).unwrap();
        connection.execute_batch("INSERT INTO employees(id,name,role,color,created_at)
            VALUES ('a','A','美容师','#000','2026-11-01T00:00:00Z'),('b','B','美容师','#000','2026-11-01T00:00:00Z');
            INSERT INTO employee_compensations(employee_id,base_salary,base_commission_rate,performance_target,
                excess_commission_rate,meal_allowance_per_day,attendance_bonus,normal_service_commission,package_service_commission,created_at)
            VALUES ('a',1800,0.1,10000,0.02,10,300,5,10,'2026-11-01'),('b',1800,0.1,10000,0.02,10,300,5,10,'2026-11-01');
            INSERT INTO members(id,name,phone,balance,principal_balance,total_recharge,join_date,last_visit)
            VALUES ('m','顾客','13800000000',1000,1000,1000,'2026-11-01','2026-11-01');
            INSERT INTO packages(id,name,price,total_uses,limit_type,validity_days,package_type,status,created_at,updated_at,commission_mode,commission_rate)
            VALUES ('water','基础水光',1980,12,'count',365,'普通','active','2026-11-01','2026-11-01','independent',0.2);
            INSERT INTO transactions(id,member_id,member_name,type,amount,balance_after,payment_method,item,employee,created_at)
            VALUES ('charge','m','顾客','recharge',1000,1000,'现金','充值','A','2026-11-02T00:00:00Z');").unwrap();
        connection
    }

    fn purchase(
        connection: &Connection,
        id: &str,
        price: f64,
        balance: f64,
        cash: f64,
        time: &str,
    ) {
        connection.execute("INSERT INTO package_purchases(id,member_id,member_name,employee,package_id,package_name,
            package_type,price,total_uses,remaining_uses,limit_type,validity_days,payment_method,balance_payment_amount,
            cash_payment_amount,status,purchased_at,commission,commission_rule_version,transaction_id,commission_mode,commission_rate)
            VALUES (?1,'m','顾客','B','water','基础水光','普通',?2,12,12,'count',365,'余额现金组合支付',?3,?4,'active',?5,0,3,'','independent',0.2)",
            params![id,price,balance,cash,time]).unwrap();
    }

    #[test]
    fn mixed_payment_and_cross_month_refund_preserve_credit_sources() {
        let connection = setup();
        purchase(
            &connection,
            "sale",
            1980.0,
            1000.0,
            980.0,
            "2026-11-03T00:00:00Z",
        );
        let data = snapshot(&connection).unwrap();
        let sale = data
            .commission_entries
            .iter()
            .find(|entry| entry.id == "package:sale")
            .unwrap();
        assert_eq!(
            (
                sale.recharge_offset,
                sale.balance_commission,
                sale.cash_commission,
                sale.commission
            ),
            (100.0, 100.0, 196.0, 296.0)
        );
        assert_eq!(sale.employees[0].employee_id, "b");
        assert_eq!(data.package_purchases[0].commission, 296.0);
        let count = data.commission_entries.len();
        assert_eq!(
            snapshot(&connection).unwrap().commission_entries.len(),
            count
        );
        connection.execute("INSERT INTO refund_records(id,refund_type,member_id,member_name,package_purchase_id,amount,balance_amount,
            cash_amount,gift_forfeited_amount,commission,employee,operator,balance_after,created_at,note)
            VALUES ('refund','package','m','顾客','sale',1782,900,882,0,0,'B','店长',900,'2026-12-01T00:00:00Z','')",[]).unwrap();
        purchase(
            &connection,
            "second",
            900.0,
            900.0,
            0.0,
            "2026-12-02T00:00:00Z",
        );
        let data = snapshot(&connection).unwrap();
        let refund = data
            .commission_entries
            .iter()
            .find(|entry| entry.id == "refund:refund")
            .unwrap();
        assert_eq!(refund.commission, -266.4);
        let second = data
            .commission_entries
            .iter()
            .find(|entry| entry.id == "package:second")
            .unwrap();
        assert_eq!((second.recharge_offset, second.commission), (90.0, 90.0));
    }

    #[test]
    fn recharge_excess_bonus_and_new_rules_do_not_rewrite_previous_entries() {
        let connection = setup();
        connection
            .execute("UPDATE transactions SET amount=10000 WHERE id='charge'", [])
            .unwrap();
        connection.execute("INSERT INTO transactions(id,member_id,member_name,type,amount,balance_after,payment_method,item,employee,created_at)
            VALUES ('charge2','m','顾客','recharge',1000,11000,'现金','充值','A','2026-11-02T01:00:00Z')",[]).unwrap();
        purchase(
            &connection,
            "general",
            10000.0,
            10000.0,
            0.0,
            "2026-11-03T00:00:00Z",
        );
        connection
            .execute(
                "UPDATE package_purchases SET commission_mode='general' WHERE id='general'",
                [],
            )
            .unwrap();
        purchase(
            &connection,
            "sale",
            1000.0,
            1000.0,
            0.0,
            "2026-11-04T00:00:00Z",
        );
        let data = snapshot(&connection).unwrap();
        let entry = data
            .commission_entries
            .iter()
            .find(|entry| entry.id == "package:sale")
            .unwrap();
        assert_eq!((entry.recharge_offset, entry.commission), (120.0, 80.0));
        connection
            .execute(
                "UPDATE employee_compensations SET base_commission_rate=0.15",
                [],
            )
            .unwrap();
        connection
            .execute("UPDATE packages SET commission_rate=0.16", [])
            .unwrap();
        let data = snapshot(&connection).unwrap();
        assert_eq!(
            data.commission_entries
                .iter()
                .find(|entry| entry.id == "package:sale")
                .unwrap()
                .commission,
            80.0
        );
        assert_eq!(
            data.commission_entries
                .iter()
                .find(|entry| entry.id == "recharge:charge2")
                .unwrap()
                .commission,
            120.0
        );
    }

    #[test]
    fn unverified_principal_causes_atomic_sale_rollback() {
        let mut connection = setup();
        connection.execute("DELETE FROM transactions", []).unwrap();
        {
            let transaction = connection.transaction().unwrap();
            purchase(
                &transaction,
                "sale",
                1000.0,
                1000.0,
                0.0,
                "2026-11-03T00:00:00Z",
            );
            assert!(sync(&transaction, false)
                .unwrap_err()
                .contains("历史充值来源不完整"));
        }
        assert!(snapshot(&connection).unwrap().package_purchases.is_empty());
    }

    #[test]
    fn real_purchase_and_refund_create_and_close_linked_gifts_atomically() {
        use crate::commands::packages::{record_package_purchase, record_package_refund};
        let mut connection = setup();
        connection.execute_batch("INSERT INTO packages(id,name,price,total_uses,limit_type,validity_days,package_type,status,created_at,updated_at)
            VALUES ('gift','基础赠送',198,1,'count',0,'普通','active','2026-11-01','2026-11-01');
            UPDATE packages SET gift_package_id='gift',gift_uses=2 WHERE id='water';").unwrap();
        record_package_purchase(
            &mut connection,
            &PackagePurchaseInput {
                member_id: "m".into(),
                package_id: "water".into(),
                employee: "B".into(),
                activated_at: chrono::Utc::now().to_rfc3339(),
                payment_method: "余额现金组合支付".into(),
                balance_payment_amount: 1000.0,
                cash_payment_amount: 980.0,
                note: String::new(),
            },
        )
        .unwrap();
        let data = snapshot(&connection).unwrap();
        assert_eq!(data.package_purchases.len(), 2);
        let parent = data
            .package_purchases
            .iter()
            .find(|item| item.parent_purchase_id.is_none())
            .unwrap();
        let child = data
            .package_purchases
            .iter()
            .find(|item| item.parent_purchase_id.is_some())
            .unwrap();
        assert_eq!(
            (parent.commission, child.commission, child.remaining_uses),
            (296.0, 0.0, 2)
        );
        assert_eq!(parent.expires_at, child.expires_at);
        assert_eq!(data.members[0].principal_balance, 0.0);
        let parent_id = parent.id.clone();
        record_package_refund(
            &mut connection,
            &PackageRefundInput {
                purchase_id: parent_id,
                single_original_price: 198.0,
                note: String::new(),
            },
            "店长",
        )
        .unwrap();
        let data = snapshot(&connection).unwrap();
        assert_eq!(data.members[0].principal_balance, 1000.0);
        assert!(data
            .package_purchases
            .iter()
            .all(|item| item.refunded_at.is_some()));
        assert_eq!(data.refund_records[0].commission, 296.0);
        assert_eq!(
            data.commission_entries
                .iter()
                .filter(|item| item.mode == "independent")
                .map(|item| item.commission)
                .sum::<f64>(),
            0.0
        );
    }

    #[test]
    fn legacy_spending_uses_gifts_before_principal_credit() {
        let connection = setup();
        connection
            .execute(
                "UPDATE transactions SET gift_amount=200,balance_after=1200 WHERE id='charge'",
                [],
            )
            .unwrap();
        connection.execute("INSERT INTO transactions(id,member_id,member_name,type,amount,balance_after,payment_method,item,employee,created_at)
            VALUES ('legacy','m','顾客','consume',150,1050,'会员余额','历史护理','A','2026-11-02T01:00:00Z')",[]).unwrap();
        purchase(
            &connection,
            "sale",
            990.0,
            990.0,
            0.0,
            "2026-11-03T00:00:00Z",
        );
        let data = snapshot(&connection).unwrap();
        let sale = data
            .commission_entries
            .iter()
            .find(|entry| entry.id == "package:sale")
            .unwrap();
        assert_eq!((sale.recharge_offset, sale.commission), (99.0, 99.0));
    }
}
