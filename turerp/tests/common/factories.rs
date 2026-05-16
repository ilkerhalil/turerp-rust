use actix_web::body::to_bytes;
use actix_web::http::StatusCode;
use actix_web::test;
use serde_json::json;

/// Builder for creating Cari records via the API
pub struct CariBuilder {
    tenant_id: i64,
    user_id: i64,
    code: String,
    name: String,
    cari_type: String,
}

impl CariBuilder {
    pub fn new(tenant_id: i64, user_id: i64) -> Self {
        let unique = uuid::Uuid::new_v4().to_string();
        Self {
            tenant_id,
            user_id,
            code: format!("Cari-{}", unique),
            name: "Test Cari".to_string(),
            cari_type: "customer".to_string(),
        }
    }

    pub fn code(mut self, code: impl Into<String>) -> Self {
        self.code = code.into();
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn cari_type(mut self, cari_type: impl Into<String>) -> Self {
        self.cari_type = cari_type.into();
        self
    }

    pub async fn create<S>(self, app: &S, token: &str) -> i64
    where
        S: actix_web::dev::Service<
            actix_web::dev::ServiceRequest,
            Response = actix_web::dev::ServiceResponse<actix_web::body::EitherBody<actix_web::body::BoxBody>>,
            Error = actix_web::Error,
        >,
    {
        let req = test::TestRequest::post()
            .uri("/api/v1/cari")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(json!({
                "code": self.code,
                "name": self.name,
                "cari_type": self.cari_type,
                "tenant_id": self.tenant_id,
                "created_by": self.user_id
            }))
            .to_request();
        let resp = test::call_service(app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED, "Cari creation failed");
        let body = to_bytes(resp.into_body()).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        json["id"].as_i64().expect("Cari response missing 'id'")
    }
}

/// Builder for creating Warehouse records via the API
pub struct WarehouseBuilder {
    tenant_id: i64,
    user_id: i64,
    code: String,
    name: String,
    company_id: i64,
}

impl WarehouseBuilder {
    pub fn new(tenant_id: i64, user_id: i64) -> Self {
        let unique = uuid::Uuid::new_v4().to_string();
        Self {
            tenant_id,
            user_id,
            code: format!("WH-{}", unique),
            name: "Test Warehouse".to_string(),
            company_id: 1,
        }
    }

    pub fn code(mut self, code: impl Into<String>) -> Self {
        self.code = code.into();
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn company_id(mut self, company_id: i64) -> Self {
        self.company_id = company_id;
        self
    }

    pub async fn create<S>(self, app: &S, token: &str) -> i64
    where
        S: actix_web::dev::Service<
            actix_web::dev::ServiceRequest,
            Response = actix_web::dev::ServiceResponse<actix_web::body::EitherBody<actix_web::body::BoxBody>>,
            Error = actix_web::Error,
        >,
    {
        let req = test::TestRequest::post()
            .uri("/api/v1/stock/warehouses")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(json!({
                "code": self.code,
                "name": self.name,
                "tenant_id": self.tenant_id,
                "company_id": self.company_id
            }))
            .to_request();
        let resp = test::call_service(app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED, "Warehouse creation failed");
        let body = to_bytes(resp.into_body()).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        json["id"].as_i64().expect("Warehouse response missing 'id'")
    }
}

/// Builder for creating Chart of Accounts records via the API
pub struct AccountBuilder {
    tenant_id: i64,
    user_id: i64,
    code: String,
    name: String,
    group: String,
    account_type: String,
    allow_posting: bool,
}

impl AccountBuilder {
    pub fn new(tenant_id: i64, _user_id: i64) -> Self {
        let unique = uuid::Uuid::new_v4().to_string();
        Self {
            tenant_id,
            user_id: _user_id,
            code: format!("ACC-{}", unique),
            name: "Test Account".to_string(),
            group: "DonenVarliklar".to_string(),
            account_type: "Asset".to_string(),
            allow_posting: true,
        }
    }

    pub fn code(mut self, code: impl Into<String>) -> Self {
        self.code = code.into();
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn group(mut self, group: impl Into<String>) -> Self {
        self.group = group.into();
        self
    }

    pub fn account_type(mut self, account_type: impl Into<String>) -> Self {
        self.account_type = account_type.into();
        self
    }

    pub fn allow_posting(mut self, allow_posting: bool) -> Self {
        self.allow_posting = allow_posting;
        self
    }

    pub async fn create<S>(self, app: &S, token: &str) -> i64
    where
        S: actix_web::dev::Service<
            actix_web::dev::ServiceRequest,
            Response = actix_web::dev::ServiceResponse<actix_web::body::EitherBody<actix_web::body::BoxBody>>,
            Error = actix_web::Error,
        >,
    {
        let req = test::TestRequest::post()
            .uri("/api/v1/chart-of-accounts")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(json!({
                "code": self.code,
                "name": self.name,
                "group": self.group,
                "account_type": self.account_type,
                "allow_posting": self.allow_posting
            }))
            .to_request();
        let resp = test::call_service(app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED, "Chart account creation failed");
        let body = to_bytes(resp.into_body()).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        json["id"].as_i64().expect("Chart account response missing 'id'")
    }
}
