//! SGK payroll service

use async_trait::async_trait;
use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use rust_decimal::Decimal;
use std::sync::Arc;

use crate::domain::hr::model::{Employee, EmployeeStatus, Payroll, PayrollStatus};
use crate::domain::hr::service::HrService;
use crate::domain::hr::sgk::calculator::{
    default_income_tax_brackets_2026, default_sgk_config_2026, PayrollCalculator,
};
use crate::domain::hr::sgk::ebildirge::{EBildirgeGenerator, EmployerInfo};
use crate::domain::hr::sgk::model::{
    CreateEmployeeBonus, CreateSgkEmployeeRegistration, EmployeeBonus, SgkConfig,
    SgkEmployeeRegistration, SgkPayrollLineItem, SgkPayrollSummary,
};
use crate::domain::hr::sgk::repository::{
    BoxEmployeeBonusRepository, BoxSgkConfigRepository, BoxSgkEmployeeRegistrationRepository,
};
use crate::error::ApiError;

/// Trait abstracting the HR data that SGK payroll service needs.
/// Implemented by [`HrService`] and can be mocked in tests.
#[async_trait]
pub trait SgkEmployeeProvider: Send + Sync {
    async fn find_employee_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<Employee>, ApiError>;
    async fn find_employees_by_tenant(&self, tenant_id: i64) -> Result<Vec<Employee>, ApiError>;
    async fn create_payroll(&self, payroll: Payroll) -> Result<Payroll, ApiError>;
    async fn find_payrolls_by_period(
        &self,
        tenant_id: i64,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Payroll>, ApiError>;
}

#[async_trait]
impl SgkEmployeeProvider for HrService {
    async fn find_employee_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<Employee>, ApiError> {
        self.employee_repo().find_by_id(id, tenant_id).await
    }

    async fn find_employees_by_tenant(&self, tenant_id: i64) -> Result<Vec<Employee>, ApiError> {
        self.employee_repo().find_by_tenant(tenant_id).await
    }

    async fn create_payroll(&self, payroll: Payroll) -> Result<Payroll, ApiError> {
        self.payroll_repo().create(payroll).await
    }

    async fn find_payrolls_by_period(
        &self,
        tenant_id: i64,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Payroll>, ApiError> {
        self.payroll_repo()
            .find_by_period(tenant_id, start, end)
            .await
    }
}

#[derive(Clone)]
pub struct SgkPayrollService {
    hr_provider: Arc<dyn SgkEmployeeProvider>,
    sgk_reg_repo: BoxSgkEmployeeRegistrationRepository,
    sgk_config_repo: BoxSgkConfigRepository,
    bonus_repo: BoxEmployeeBonusRepository,
}

impl SgkPayrollService {
    pub fn new(
        hr_provider: Arc<dyn SgkEmployeeProvider>,
        sgk_reg_repo: BoxSgkEmployeeRegistrationRepository,
        sgk_config_repo: BoxSgkConfigRepository,
        bonus_repo: BoxEmployeeBonusRepository,
    ) -> Self {
        Self {
            hr_provider,
            sgk_reg_repo,
            sgk_config_repo,
            bonus_repo,
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn register_employee(
        &self,
        create: CreateSgkEmployeeRegistration,
    ) -> Result<SgkEmployeeRegistration, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        let reg = SgkEmployeeRegistration {
            id: 0,
            employee_id: create.employee_id,
            tenant_id: create.tenant_id,
            tc_kimlik_no: create.tc_kimlik_no,
            sgk_sicil_no: create.sgk_sicil_no,
            workplace_code: create.workplace_code,
            profession_code: create.profession_code,
            registration_date: create.registration_date,
            termination_date: None,
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.sgk_reg_repo.create(reg).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_or_create_config(
        &self,
        tenant_id: i64,
        year: i32,
    ) -> Result<SgkConfig, ApiError> {
        match self
            .sgk_config_repo
            .find_by_tenant_and_year(tenant_id, year)
            .await?
        {
            Some(config) => Ok(config),
            None => {
                let mut config = default_sgk_config_2026();
                config.tenant_id = tenant_id;
                config.year = year;
                config.created_at = Utc::now();
                config.updated_at = Utc::now();
                self.sgk_config_repo.create(config).await
            }
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn calculate_sgk_payroll(
        &self,
        tenant_id: i64,
        employee_id: i64,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> Result<Payroll, ApiError> {
        let employee = self
            .hr_provider
            .find_employee_by_id(employee_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Employee not found".to_string()))?;

        let year = period_start.year();
        let month = period_start.month() as i32;

        let bonuses = self
            .bonus_repo
            .find_by_employee_and_year_month(employee_id, year, month)
            .await?;
        let bonus_total: Decimal = bonuses.iter().map(|b| b.amount).sum();

        let config = self.get_or_create_config(tenant_id, year).await?;
        let brackets = default_income_tax_brackets_2026();
        let calculator = PayrollCalculator::new(config, brackets);

        let marital_status = employee.marital_status.as_deref().unwrap_or("single");
        let line = calculator.gross_to_net(
            employee.salary,
            bonus_total,
            marital_status,
            employee.children_count,
            employee.spouse_working,
        );

        let deductions = line.sgk_premium_worker
            + line.unemployment_premium_worker
            + line.income_tax
            + line.stamp_tax;

        let payroll = Payroll {
            id: 0,
            tenant_id,
            employee_id,
            period_start,
            period_end,
            basic_salary: employee.salary,
            overtime_hours: Decimal::ZERO,
            overtime_pay: Decimal::ZERO,
            bonuses: bonus_total,
            gross_salary: line.gross_salary,
            sgk_premium: line.sgk_premium_worker,
            unemployment_premium: line.unemployment_premium_worker,
            income_tax: line.income_tax,
            stamp_tax: line.stamp_tax,
            agi: line.agi,
            sgk_earnings_base: line.sgk_earnings_base,
            total_employer_cost: line.employer_cost,
            deductions,
            net_salary: line.net_salary,
            status: PayrollStatus::Calculated,
            paid_at: None,
            created_at: Utc::now(),
            deleted_at: None,
            deleted_by: None,
        };

        self.hr_provider.create_payroll(payroll).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn generate_ebildirge(
        &self,
        tenant_id: i64,
        year: i32,
        month: i32,
        employer_info: EmployerInfo,
    ) -> Result<String, ApiError> {
        let period_start = Utc
            .with_ymd_and_hms(year, month as u32, 1, 0, 0, 0)
            .single()
            .ok_or_else(|| ApiError::BadRequest("Invalid period".to_string()))?;
        let (next_year, next_month) = if month == 12 {
            (year + 1, 1)
        } else {
            (year, month + 1)
        };
        let period_end = Utc
            .with_ymd_and_hms(next_year, next_month as u32, 1, 0, 0, 0)
            .single()
            .ok_or_else(|| ApiError::BadRequest("Invalid period".to_string()))?
            - Duration::seconds(1);

        let employees = self.hr_provider.find_employees_by_tenant(tenant_id).await?;

        let mut employee_data = Vec::new();

        for employee in employees {
            if employee.status != EmployeeStatus::Active {
                continue;
            }

            let payrolls = self
                .hr_provider
                .find_payrolls_by_period(tenant_id, period_start, period_end)
                .await?;
            let payroll = match payrolls.into_iter().find(|p| p.employee_id == employee.id) {
                Some(p) => p,
                None => {
                    self.calculate_sgk_payroll(tenant_id, employee.id, period_start, period_end)
                        .await?
                }
            };

            let mut employee = employee.clone();
            if let Ok(Some(reg)) = self.sgk_reg_repo.find_active_by_employee(employee.id).await {
                employee.sgk_sicil_no = Some(reg.sgk_sicil_no);
            }

            let line_item = SgkPayrollLineItem {
                employee_id: employee.id,
                gross_salary: payroll.gross_salary,
                sgk_earnings_base: payroll.sgk_earnings_base,
                sgk_premium_worker: payroll.sgk_premium,
                unemployment_premium_worker: payroll.unemployment_premium,
                income_tax_base: payroll.gross_salary
                    - payroll.sgk_premium
                    - payroll.unemployment_premium,
                income_tax: payroll.income_tax,
                stamp_tax: payroll.stamp_tax,
                agi: payroll.agi,
                net_salary: payroll.net_salary,
                employer_cost: payroll.total_employer_cost,
            };

            employee_data.push((employee, line_item));
        }

        let xml = EBildirgeGenerator::generate_monthly_declaration(
            &employer_info,
            year,
            month,
            &employee_data,
        );

        Ok(xml)
    }

    #[tracing::instrument(skip(self))]
    pub async fn add_bonus(&self, create: CreateEmployeeBonus) -> Result<EmployeeBonus, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        // Parent-ownership precheck: the employee must belong to the caller's
        // tenant, else a tenant-A caller could post a bonus against tenant-B's
        // employee (cross-tenant orphan write). The handler forces
        // `create.tenant_id` from the auth token, so it is the auth-derived
        // tenant here. Also yields a clean NotFound for a bogus employee_id
        // instead of an FK violation.
        self.hr_provider
            .find_employee_by_id(create.employee_id, create.tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Employee not found".to_string()))?;
        let bonus = EmployeeBonus {
            id: 0,
            employee_id: create.employee_id,
            tenant_id: create.tenant_id,
            bonus_type: create.bonus_type,
            amount: create.amount,
            bonus_month: create.bonus_month,
            bonus_year: create.bonus_year,
            description: create.description,
            created_at: Utc::now(),
        };
        self.bonus_repo.create(bonus).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_payroll_summary(
        &self,
        tenant_id: i64,
        year: i32,
        month: i32,
    ) -> Result<SgkPayrollSummary, ApiError> {
        let period_start = Utc
            .with_ymd_and_hms(year, month as u32, 1, 0, 0, 0)
            .single()
            .ok_or_else(|| ApiError::BadRequest("Invalid period".to_string()))?;
        let (next_year, next_month) = if month == 12 {
            (year + 1, 1)
        } else {
            (year, month + 1)
        };
        let period_end = Utc
            .with_ymd_and_hms(next_year, next_month as u32, 1, 0, 0, 0)
            .single()
            .ok_or_else(|| ApiError::BadRequest("Invalid period".to_string()))?
            - Duration::seconds(1);

        let payrolls = self
            .hr_provider
            .find_payrolls_by_period(tenant_id, period_start, period_end)
            .await?;

        let mut total_gross = Decimal::ZERO;
        let mut total_sgk_premium_worker = Decimal::ZERO;
        let mut total_unemployment_worker = Decimal::ZERO;
        let mut total_income_tax = Decimal::ZERO;
        let mut total_stamp_tax = Decimal::ZERO;
        let mut total_agi = Decimal::ZERO;
        let mut total_net = Decimal::ZERO;
        let mut total_employer_cost = Decimal::ZERO;
        let mut line_items = Vec::new();

        for payroll in &payrolls {
            total_gross += payroll.gross_salary;
            total_sgk_premium_worker += payroll.sgk_premium;
            total_unemployment_worker += payroll.unemployment_premium;
            total_income_tax += payroll.income_tax;
            total_stamp_tax += payroll.stamp_tax;
            total_agi += payroll.agi;
            total_net += payroll.net_salary;
            total_employer_cost += payroll.total_employer_cost;

            line_items.push(SgkPayrollLineItem {
                employee_id: payroll.employee_id,
                gross_salary: payroll.gross_salary,
                sgk_earnings_base: payroll.sgk_earnings_base,
                sgk_premium_worker: payroll.sgk_premium,
                unemployment_premium_worker: payroll.unemployment_premium,
                income_tax_base: payroll.gross_salary
                    - payroll.sgk_premium
                    - payroll.unemployment_premium,
                income_tax: payroll.income_tax,
                stamp_tax: payroll.stamp_tax,
                agi: payroll.agi,
                net_salary: payroll.net_salary,
                employer_cost: payroll.total_employer_cost,
            });
        }

        Ok(SgkPayrollSummary {
            tenant_id,
            period_year: year,
            period_month: month,
            total_gross,
            total_sgk_premium_worker,
            total_unemployment_worker,
            total_income_tax,
            total_stamp_tax,
            total_agi,
            total_net,
            total_employer_cost,
            employee_count: line_items.len() as i32,
            line_items,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::hr::model::CreateEmployee;
    use crate::domain::hr::repository::{
        InMemoryAttendanceRepository, InMemoryEmployeeRepository, InMemoryLeaveRequestRepository,
        InMemoryLeaveTypeRepository, InMemoryPayrollRepository,
    };
    use crate::domain::hr::sgk::repository::{
        InMemoryEmployeeBonusRepository, InMemorySgkConfigRepository,
        InMemorySgkEmployeeRegistrationRepository,
    };
    use chrono::Utc;
    use rust_decimal::Decimal;
    use std::sync::Arc;

    fn create_service() -> (HrService, SgkPayrollService) {
        let employee_repo = Arc::new(InMemoryEmployeeRepository::new()) as _;
        let attendance_repo = Arc::new(InMemoryAttendanceRepository::new()) as _;
        let leave_request_repo = Arc::new(InMemoryLeaveRequestRepository::new()) as _;
        let leave_type_repo = Arc::new(InMemoryLeaveTypeRepository::new()) as _;
        let payroll_repo = Arc::new(InMemoryPayrollRepository::new()) as _;

        let hr_service = HrService::new(
            employee_repo,
            attendance_repo,
            leave_request_repo,
            leave_type_repo,
            payroll_repo,
        );

        let sgk_reg_repo = Arc::new(InMemorySgkEmployeeRegistrationRepository::new()) as _;
        let sgk_config_repo = Arc::new(InMemorySgkConfigRepository::new()) as _;
        let bonus_repo = Arc::new(InMemoryEmployeeBonusRepository::new()) as _;

        let sgk_payroll_service = SgkPayrollService::new(
            Arc::new(hr_service.clone()),
            sgk_reg_repo,
            sgk_config_repo,
            bonus_repo,
        );

        (hr_service, sgk_payroll_service)
    }

    fn sample_create_employee() -> CreateEmployee {
        CreateEmployee {
            tenant_id: 1,
            company_id: 1,
            user_id: None,
            employee_number: "EMP001".to_string(),
            first_name: "John".to_string(),
            last_name: "Doe".to_string(),
            email: "john@example.com".to_string(),
            phone: None,
            department: None,
            position: None,
            hire_date: Utc::now(),
            salary: Decimal::new(5000000, 2),
            tc_kimlik_no: "12345678901".to_string(),
            children_count: 0,
        }
    }

    #[tokio::test]
    async fn test_register_employee() {
        let (_hr, service) = create_service();
        let create = CreateSgkEmployeeRegistration {
            employee_id: 1,
            tenant_id: 1,
            tc_kimlik_no: "12345678901".to_string(),
            sgk_sicil_no: "SGK001".to_string(),
            workplace_code: "WP001".to_string(),
            profession_code: "PROF001".to_string(),
            registration_date: Utc::now(),
        };
        let result = service.register_employee(create).await.unwrap();
        assert_eq!(result.tc_kimlik_no, "12345678901");
        assert_eq!(result.sgk_sicil_no, "SGK001");
        assert!(result.is_active);
    }

    fn sample_bonus(employee_id: i64, tenant_id: i64) -> CreateEmployeeBonus {
        CreateEmployeeBonus {
            employee_id,
            tenant_id,
            bonus_type: "performance".to_string(),
            amount: Decimal::new(50000, 2),
            bonus_month: 6,
            bonus_year: 2026,
            description: None,
        }
    }

    #[tokio::test]
    async fn test_add_bonus() {
        let (hr, service) = create_service();
        let emp = hr.create_employee(sample_create_employee()).await.unwrap();
        let result = service.add_bonus(sample_bonus(emp.id, 1)).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().employee_id, emp.id);
    }

    #[tokio::test]
    async fn test_add_bonus_rejects_foreign_employee() {
        let (hr, service) = create_service();
        // employee exists only on tenant 1; a tenant-2 caller cannot post a
        // bonus against it (cross-tenant orphan write).
        let emp = hr.create_employee(sample_create_employee()).await.unwrap();
        let result = service.add_bonus(sample_bonus(emp.id, 2)).await;
        assert!(matches!(result, Err(ApiError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_calculate_sgk_payroll() {
        let (hr, service) = create_service();
        let emp = hr.create_employee(sample_create_employee()).await.unwrap();

        let period_start = Utc.with_ymd_and_hms(2024, 6, 1, 0, 0, 0).unwrap();
        let period_end = Utc.with_ymd_and_hms(2024, 6, 30, 23, 59, 59).unwrap();

        let payroll = service
            .calculate_sgk_payroll(1, emp.id, period_start, period_end)
            .await
            .unwrap();

        assert_eq!(payroll.basic_salary, Decimal::new(5000000, 2));
        assert!(payroll.net_salary < payroll.gross_salary);
        assert!(payroll.deductions > Decimal::ZERO);
        assert!(payroll.sgk_premium > Decimal::ZERO);
        assert!(payroll.unemployment_premium > Decimal::ZERO);
    }

    #[tokio::test]
    async fn test_generate_ebildirge() {
        let (hr, service) = create_service();
        let emp = hr.create_employee(sample_create_employee()).await.unwrap();

        service
            .register_employee(CreateSgkEmployeeRegistration {
                employee_id: emp.id,
                tenant_id: 1,
                tc_kimlik_no: "12345678901".to_string(),
                sgk_sicil_no: "SGK001".to_string(),
                workplace_code: "WP001".to_string(),
                profession_code: "PROF001".to_string(),
                registration_date: Utc::now(),
            })
            .await
            .unwrap();

        let period_start = Utc.with_ymd_and_hms(2024, 6, 1, 0, 0, 0).unwrap();
        let period_end = Utc.with_ymd_and_hms(2024, 6, 30, 23, 59, 59).unwrap();
        service
            .calculate_sgk_payroll(1, emp.id, period_start, period_end)
            .await
            .unwrap();

        let employer_info = EmployerInfo {
            company_name: "Test Co".to_string(),
            tax_number: "1234567890".to_string(),
            sgk_workplace_code: "WP001".to_string(),
            address: "Test Address".to_string(),
            phone: "5551234567".to_string(),
        };

        let xml = service
            .generate_ebildirge(1, 2024, 6, employer_info)
            .await
            .unwrap();

        assert!(xml.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(xml.contains("<AylikPrimHizmetBelgesi"));
        assert!(xml.contains("<IsyeriSicilNo>WP001</IsyeriSicilNo>"));
        assert!(xml.contains("<IsyeriAdi>Test Co</IsyeriAdi>"));
        assert!(xml.contains("<Yil>2024</Yil>"));
        assert!(xml.contains("<Ay>6</Ay>"));
        assert!(xml.contains("<TCKimlikNo>12345678901</TCKimlikNo>"));
        assert!(xml.contains("<SicilNo>SGK001</SicilNo>"));
        assert!(xml.contains("<Ad>John</Ad>"));
        assert!(xml.contains("<Soyad>Doe</Soyad>"));
        assert!(xml.contains("<Calisanlar>"));
        assert!(xml.contains("<Toplamlar>"));
        assert!(xml.contains("<CalisanSayisi>1</CalisanSayisi>"));
    }
}
