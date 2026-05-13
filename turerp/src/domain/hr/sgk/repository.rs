//! SGK repository traits and in-memory implementations

use async_trait::async_trait;
use parking_lot::Mutex;
use std::sync::Arc;

use crate::domain::hr::sgk::model::{EmployeeBonus, SgkConfig, SgkEmployeeRegistration};
use crate::error::ApiError;

/// Repository trait for SGK employee registration operations
#[async_trait]
pub trait SgkEmployeeRegistrationRepository: Send + Sync {
    async fn create(
        &self,
        reg: SgkEmployeeRegistration,
    ) -> Result<SgkEmployeeRegistration, ApiError>;
    async fn find_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<SgkEmployeeRegistration>, ApiError>;
    async fn find_by_employee(
        &self,
        employee_id: i64,
    ) -> Result<Vec<SgkEmployeeRegistration>, ApiError>;
    async fn find_active_by_employee(
        &self,
        employee_id: i64,
    ) -> Result<Option<SgkEmployeeRegistration>, ApiError>;
    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        reg: SgkEmployeeRegistration,
    ) -> Result<SgkEmployeeRegistration, ApiError>;
    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

/// Repository trait for SGK config operations
#[async_trait]
pub trait SgkConfigRepository: Send + Sync {
    async fn create(&self, config: SgkConfig) -> Result<SgkConfig, ApiError>;
    async fn find_by_tenant_and_year(
        &self,
        tenant_id: i64,
        year: i32,
    ) -> Result<Option<SgkConfig>, ApiError>;
    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        config: SgkConfig,
    ) -> Result<SgkConfig, ApiError>;
}

/// Repository trait for employee bonus operations
#[async_trait]
pub trait EmployeeBonusRepository: Send + Sync {
    async fn create(&self, bonus: EmployeeBonus) -> Result<EmployeeBonus, ApiError>;
    async fn find_by_employee_and_year_month(
        &self,
        employee_id: i64,
        year: i32,
        month: i32,
    ) -> Result<Vec<EmployeeBonus>, ApiError>;
    async fn delete(&self, id: i64) -> Result<(), ApiError>;
}

/// Type aliases
pub type BoxSgkEmployeeRegistrationRepository = Arc<dyn SgkEmployeeRegistrationRepository>;
pub type BoxSgkConfigRepository = Arc<dyn SgkConfigRepository>;
pub type BoxEmployeeBonusRepository = Arc<dyn EmployeeBonusRepository>;

/// Inner state for InMemorySgkEmployeeRegistrationRepository
struct InMemorySgkEmployeeRegistrationInner {
    records: std::collections::HashMap<i64, SgkEmployeeRegistration>,
    next_id: i64,
}

/// In-memory SGK employee registration repository
pub struct InMemorySgkEmployeeRegistrationRepository {
    inner: Mutex<InMemorySgkEmployeeRegistrationInner>,
}

impl InMemorySgkEmployeeRegistrationRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemorySgkEmployeeRegistrationInner {
                records: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemorySgkEmployeeRegistrationRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SgkEmployeeRegistrationRepository for InMemorySgkEmployeeRegistrationRepository {
    async fn create(
        &self,
        reg: SgkEmployeeRegistration,
    ) -> Result<SgkEmployeeRegistration, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;
        let mut r = reg;
        r.id = id;
        inner.records.insert(id, r.clone());
        Ok(r)
    }

    async fn find_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<SgkEmployeeRegistration>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .records
            .get(&id)
            .filter(|r| r.tenant_id == tenant_id)
            .cloned())
    }

    async fn find_by_employee(
        &self,
        employee_id: i64,
    ) -> Result<Vec<SgkEmployeeRegistration>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .records
            .values()
            .filter(|r| r.employee_id == employee_id)
            .cloned()
            .collect())
    }

    async fn find_active_by_employee(
        &self,
        employee_id: i64,
    ) -> Result<Option<SgkEmployeeRegistration>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .records
            .values()
            .find(|r| r.employee_id == employee_id && r.is_active)
            .cloned())
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        reg: SgkEmployeeRegistration,
    ) -> Result<SgkEmployeeRegistration, ApiError> {
        let mut inner = self.inner.lock();
        let existing = inner
            .records
            .get(&id)
            .ok_or_else(|| ApiError::NotFound(format!("SGK registration {} not found", id)))?;
        if existing.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!(
                "SGK registration {} not found",
                id
            )));
        }
        let mut r = reg;
        r.id = id;
        r.tenant_id = tenant_id;
        inner.records.insert(id, r.clone());
        Ok(r)
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let existing = inner
            .records
            .get(&id)
            .ok_or_else(|| ApiError::NotFound(format!("SGK registration {} not found", id)))?;
        if existing.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!(
                "SGK registration {} not found",
                id
            )));
        }
        inner.records.remove(&id);
        Ok(())
    }
}

/// Inner state for InMemorySgkConfigRepository
struct InMemorySgkConfigInner {
    records: std::collections::HashMap<i64, SgkConfig>,
    next_id: i64,
}

/// In-memory SGK config repository
pub struct InMemorySgkConfigRepository {
    inner: Mutex<InMemorySgkConfigInner>,
}

impl InMemorySgkConfigRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemorySgkConfigInner {
                records: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemorySgkConfigRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SgkConfigRepository for InMemorySgkConfigRepository {
    async fn create(&self, config: SgkConfig) -> Result<SgkConfig, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;
        let mut c = config;
        c.id = id;
        inner.records.insert(id, c.clone());
        Ok(c)
    }

    async fn find_by_tenant_and_year(
        &self,
        tenant_id: i64,
        year: i32,
    ) -> Result<Option<SgkConfig>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .records
            .values()
            .find(|c| c.tenant_id == tenant_id && c.year == year)
            .cloned())
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        config: SgkConfig,
    ) -> Result<SgkConfig, ApiError> {
        let mut inner = self.inner.lock();
        let existing = inner
            .records
            .get(&id)
            .ok_or_else(|| ApiError::NotFound(format!("SGK config {} not found", id)))?;
        if existing.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("SGK config {} not found", id)));
        }
        let mut c = config;
        c.id = id;
        c.tenant_id = tenant_id;
        inner.records.insert(id, c.clone());
        Ok(c)
    }
}

/// Inner state for InMemoryEmployeeBonusRepository
struct InMemoryEmployeeBonusInner {
    records: std::collections::HashMap<i64, EmployeeBonus>,
    next_id: i64,
}

/// In-memory employee bonus repository
pub struct InMemoryEmployeeBonusRepository {
    inner: Mutex<InMemoryEmployeeBonusInner>,
}

impl InMemoryEmployeeBonusRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryEmployeeBonusInner {
                records: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryEmployeeBonusRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EmployeeBonusRepository for InMemoryEmployeeBonusRepository {
    async fn create(&self, bonus: EmployeeBonus) -> Result<EmployeeBonus, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;
        let mut b = bonus;
        b.id = id;
        inner.records.insert(id, b.clone());
        Ok(b)
    }

    async fn find_by_employee_and_year_month(
        &self,
        employee_id: i64,
        year: i32,
        month: i32,
    ) -> Result<Vec<EmployeeBonus>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .records
            .values()
            .filter(|b| {
                b.employee_id == employee_id && b.bonus_year == year && b.bonus_month == month
            })
            .cloned()
            .collect())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        inner
            .records
            .remove(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Employee bonus {} not found", id)))?;
        Ok(())
    }
}
