use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuthUser {
    pub(crate) id: String,
    pub(crate) username: String,
    pub(crate) display_name: String,
    pub(crate) role: String,
    pub(crate) employee_id: Option<String>,
    pub(crate) status: String,
    pub(crate) must_change_password: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuthResponse {
    pub(crate) token: String,
    pub(crate) user: AuthUser,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AccountRecord {
    pub(crate) id: String,
    pub(crate) username: String,
    pub(crate) display_name: String,
    pub(crate) role: String,
    pub(crate) employee_id: Option<String>,
    pub(crate) status: String,
    pub(crate) must_change_password: bool,
    pub(crate) created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppSnapshot {
    pub(crate) members: Vec<Member>,
    pub(crate) transactions: Vec<TransactionRecord>,
    pub(crate) services: Vec<ServiceRecord>,
    pub(crate) projects: Vec<ProjectDefinition>,
    pub(crate) employees: Vec<Employee>,
    pub(crate) employee_compensations: Vec<EmployeeCompensation>,
    pub(crate) employee_status_events: Vec<EmployeeStatusEvent>,
    pub(crate) commission_configs: Vec<CommissionConfig>,
    pub(crate) attendance_records: Vec<AttendanceRecord>,
    pub(crate) packages: Vec<PackageDefinition>,
    pub(crate) package_purchases: Vec<PackagePurchase>,
    pub(crate) package_consumptions: Vec<PackageConsumption>,
    pub(crate) appointments: Vec<Appointment>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Member {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) phone: String,
    pub(crate) balance: f64,
    pub(crate) principal_balance: f64,
    pub(crate) gift_balance: f64,
    pub(crate) total_recharge: f64,
    pub(crate) total_consumption: f64,
    pub(crate) join_date: String,
    pub(crate) last_visit: String,
    pub(crate) status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TransactionRecord {
    pub(crate) id: String,
    pub(crate) member_id: Option<String>,
    pub(crate) member_name: String,
    pub(crate) r#type: String,
    pub(crate) amount: f64,
    pub(crate) gift_amount: f64,
    pub(crate) balance_after: f64,
    pub(crate) payment_method: String,
    pub(crate) item: String,
    pub(crate) employee: String,
    pub(crate) commission: f64,
    pub(crate) commission_rule_version: i64,
    pub(crate) created_at: String,
    pub(crate) note: String,
    pub(crate) status: String,
    pub(crate) source_type: String,
    pub(crate) source_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServiceRecord {
    pub(crate) id: String,
    pub(crate) member_id: Option<String>,
    pub(crate) member_name: String,
    pub(crate) employee: String,
    pub(crate) service_name: String,
    pub(crate) service_type: String,
    pub(crate) duration: i64,
    pub(crate) amount: f64,
    pub(crate) commission: f64,
    pub(crate) commission_rule_version: i64,
    pub(crate) package_purchase_id: Option<String>,
    pub(crate) project_id: Option<String>,
    pub(crate) balance_payment_amount: f64,
    pub(crate) external_payment_amount: f64,
    pub(crate) payment_method: String,
    pub(crate) gift_deduction: f64,
    pub(crate) principal_deduction: f64,
    pub(crate) transaction_id: Option<String>,
    pub(crate) created_at: String,
    pub(crate) status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectDefinition {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) duration: i64,
    pub(crate) price: f64,
    pub(crate) status: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Employee {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) role: String,
    pub(crate) status: String,
    pub(crate) color: String,
    pub(crate) created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EmployeeStatusEvent {
    pub(crate) id: String,
    pub(crate) employee_id: String,
    pub(crate) status: String,
    pub(crate) changed_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EmployeeCompensation {
    pub(crate) employee_id: String,
    pub(crate) base_salary: f64,
    pub(crate) base_commission_rate: f64,
    pub(crate) performance_target: f64,
    pub(crate) excess_commission_rate: f64,
    pub(crate) meal_allowance_per_day: f64,
    pub(crate) attendance_bonus: f64,
    pub(crate) normal_service_commission: f64,
    pub(crate) package_service_commission: f64,
    pub(crate) created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommissionConfig {
    pub(crate) id: String,
    pub(crate) effective_month: String,
    pub(crate) recharge_rate: f64,
    pub(crate) package_purchase_rate: f64,
    pub(crate) package_service_amount: f64,
    pub(crate) normal_service_amount: f64,
    pub(crate) base_salary: f64,
    pub(crate) created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AttendanceRecord {
    pub(crate) id: String,
    pub(crate) employee_id: String,
    pub(crate) month: String,
    pub(crate) rest_days: i64,
    pub(crate) updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackageDefinition {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) price: f64,
    pub(crate) total_uses: i64,
    pub(crate) limit_type: String,
    pub(crate) validity_days: i64,
    pub(crate) package_type: String,
    pub(crate) status: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackagePurchase {
    pub(crate) id: String,
    pub(crate) member_id: String,
    pub(crate) member_name: String,
    pub(crate) employee: String,
    pub(crate) package_id: String,
    pub(crate) package_name: String,
    pub(crate) package_type: String,
    pub(crate) price: f64,
    pub(crate) total_uses: i64,
    pub(crate) remaining_uses: i64,
    pub(crate) limit_type: String,
    pub(crate) validity_days: i64,
    pub(crate) activated_at: Option<String>,
    pub(crate) expires_at: Option<String>,
    pub(crate) payment_method: String,
    pub(crate) balance_payment_amount: f64,
    pub(crate) cash_payment_amount: f64,
    pub(crate) status: String,
    pub(crate) purchased_at: String,
    pub(crate) last_consumed_at: Option<String>,
    pub(crate) commission: f64,
    pub(crate) commission_rule_version: i64,
    pub(crate) transaction_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackageConsumption {
    pub(crate) id: String,
    pub(crate) purchase_id: String,
    pub(crate) employee: String,
    pub(crate) duration: i64,
    pub(crate) remaining_after: i64,
    pub(crate) created_at: String,
    pub(crate) note: String,
    pub(crate) service_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Appointment {
    pub(crate) id: String,
    pub(crate) customer_type: String,
    pub(crate) member_id: Option<String>,
    pub(crate) customer_name: String,
    pub(crate) customer_phone: String,
    pub(crate) employee: String,
    pub(crate) service_type: String,
    pub(crate) service_name: String,
    pub(crate) project_id: Option<String>,
    pub(crate) package_purchase_id: Option<String>,
    pub(crate) starts_at: String,
    pub(crate) duration: i64,
    pub(crate) status: String,
    pub(crate) note: String,
    pub(crate) created_by: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) completed_service_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EmployeeSalary {
    pub(crate) employee_id: String,
    pub(crate) name: String,
    pub(crate) recharge_performance: f64,
    pub(crate) package_purchase_performance: f64,
    pub(crate) total_performance: f64,
    pub(crate) package_service_count: i64,
    pub(crate) normal_service_count: i64,
    pub(crate) commission: f64,
    pub(crate) base_salary: f64,
    pub(crate) meal_allowance: f64,
    pub(crate) attendance_bonus: f64,
    pub(crate) total_income: f64,
    pub(crate) rest_days: i64,
    pub(crate) active_days: i64,
    pub(crate) work_days: i64,
    pub(crate) meal_days: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemberInput {
    pub(crate) name: String,
    pub(crate) phone: String,
    pub(crate) initial_balance: f64,
    #[serde(default)]
    pub(crate) gift_amount: f64,
    pub(crate) employee: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TransactionInput {
    pub(crate) member_id: String,
    pub(crate) r#type: String,
    pub(crate) amount: f64,
    #[serde(default)]
    pub(crate) gift_amount: f64,
    pub(crate) payment_method: String,
    pub(crate) item: String,
    pub(crate) employee: String,
    pub(crate) note: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServiceInput {
    pub(crate) request_id: String,
    pub(crate) member_id: String,
    pub(crate) guest_name: String,
    pub(crate) employee: String,
    pub(crate) project_id: String,
    pub(crate) external_payment_method: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProjectDefinitionInput {
    pub(crate) name: String,
    pub(crate) duration: i64,
    pub(crate) price: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EmployeeInput {
    pub(crate) name: String,
    pub(crate) role: String,
    pub(crate) color: String,
    pub(crate) base_salary: f64,
    pub(crate) base_commission_rate: f64,
    pub(crate) performance_target: f64,
    pub(crate) excess_commission_rate: f64,
    pub(crate) meal_allowance_per_day: f64,
    pub(crate) attendance_bonus: f64,
    pub(crate) normal_service_commission: f64,
    pub(crate) package_service_commission: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AccountInput {
    pub(crate) username: String,
    pub(crate) display_name: String,
    pub(crate) role: String,
    pub(crate) employee_id: Option<String>,
    pub(crate) password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommissionConfigInput {
    pub(crate) effective_month: String,
    pub(crate) recharge_rate: f64,
    pub(crate) package_purchase_rate: f64,
    pub(crate) package_service_amount: f64,
    pub(crate) normal_service_amount: f64,
    pub(crate) base_salary: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AttendanceInput {
    pub(crate) employee_id: String,
    pub(crate) month: String,
    pub(crate) rest_days: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackageDefinitionInput {
    pub(crate) name: String,
    pub(crate) price: f64,
    pub(crate) total_uses: i64,
    pub(crate) limit_type: String,
    pub(crate) validity_days: i64,
    pub(crate) package_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackagePurchaseInput {
    pub(crate) member_id: String,
    pub(crate) employee: String,
    pub(crate) package_id: String,
    #[serde(default)]
    pub(crate) activated_at: String,
    pub(crate) payment_method: String,
    #[serde(default)]
    pub(crate) balance_payment_amount: f64,
    #[serde(default)]
    pub(crate) cash_payment_amount: f64,
}

fn default_service_duration() -> i64 {
    60
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackageConsumptionInput {
    pub(crate) purchase_id: String,
    pub(crate) employee: String,
    #[serde(default = "default_service_duration")]
    pub(crate) duration: i64,
    #[serde(default)]
    pub(crate) note: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppointmentInput {
    pub(crate) customer_type: String,
    pub(crate) member_id: String,
    pub(crate) guest_name: String,
    pub(crate) guest_phone: String,
    pub(crate) employee: String,
    pub(crate) service_type: String,
    pub(crate) project_id: String,
    pub(crate) package_purchase_id: String,
    pub(crate) starts_at: String,
    pub(crate) duration: i64,
    pub(crate) note: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppointmentCompletionInput {
    pub(crate) appointment_id: String,
    pub(crate) duration: i64,
    pub(crate) external_payment_method: String,
}
