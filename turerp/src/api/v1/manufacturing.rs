//! Manufacturing API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::common::MessageResponse;
use crate::domain::manufacturing::model::{
    CreateBillOfMaterials, CreateBillOfMaterialsLine, CreateInspection, CreateNonConformanceReport,
    CreateRouting, CreateRoutingOperation, CreateWorkOrder, CreateWorkOrderMaterial,
    CreateWorkOrderOperation, UpdateInspection, UpdateNonConformanceReport, WorkOrderStatus,
};
use crate::domain::manufacturing::service::{ManufacturingService, QualityControlService};
use crate::error::{ApiError, ApiResult};
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

// --- Work Orders ---

/// Create work order (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/manufacturing/work-orders", tag = "Manufacturing",
    request_body = CreateWorkOrder,
    responses((status = 201, description = "Work order created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_work_order(
    admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    payload: web::Json<CreateWorkOrder>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    match mfg_service.create_work_order(create).await {
        Ok(order) => Ok(HttpResponse::Created().json(order)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get all work orders
#[utoipa::path(
    get, path = "/api/v1/manufacturing/work-orders", tag = "Manufacturing",
    responses((status = 200, description = "List of work orders")),
    security(("bearer_auth" = []))
)]
pub async fn get_work_orders(
    auth_user: AuthUser,
    mfg_service: web::Data<ManufacturingService>,
    query: web::Query<PaginationParams>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let pagination = query.into_inner();
    if let Err(e) = pagination.validate() {
        let err = ApiError::Validation(e.to_string());
        return Ok(err.to_http_response(i18n, locale.as_str()));
    }
    match mfg_service
        .get_work_orders_by_tenant_paginated(
            auth_user.0.tenant_id,
            pagination.page,
            pagination.per_page,
        )
        .await
    {
        Ok(result) => Ok(HttpResponse::Ok().json(result)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get work order by ID
#[utoipa::path(
    get, path = "/api/v1/manufacturing/work-orders/{id}", tag = "Manufacturing",
    params(("id" = i64, Path, description = "Work order ID")),
    responses((status = 200, description = "Work order found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_work_order(
    auth_user: AuthUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match mfg_service
        .get_work_order(*path, auth_user.0.tenant_id)
        .await
    {
        Ok(order) => Ok(HttpResponse::Ok().json(order)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Update work order status (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/manufacturing/work-orders/{id}/status", tag = "Manufacturing",
    params(("id" = i64, Path, description = "Work order ID")),
    request_body = UpdateWorkOrderStatusRequest,
    responses((status = 200, description = "Status updated"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn update_work_order_status(
    _admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateWorkOrderStatusRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match mfg_service
        .update_work_order_status(*path, payload.into_inner().status)
        .await
    {
        Ok(order) => Ok(HttpResponse::Ok().json(order)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Add work order operation (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/manufacturing/work-orders/operations", tag = "Manufacturing",
    request_body = CreateWorkOrderOperation,
    responses((status = 201, description = "Operation added"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn add_work_order_operation(
    _admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    payload: web::Json<CreateWorkOrderOperation>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match mfg_service
        .add_work_order_operation(payload.into_inner())
        .await
    {
        Ok(op) => Ok(HttpResponse::Created().json(op)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get work order operations
#[utoipa::path(
    get, path = "/api/v1/manufacturing/work-orders/{work_order_id}/operations", tag = "Manufacturing",
    params(("work_order_id" = i64, Path, description = "Work order ID")),
    responses((status = 200, description = "Operations for work order")),
    security(("bearer_auth" = []))
)]
pub async fn get_work_order_operations(
    _auth_user: AuthUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match mfg_service.get_work_order_operations(*path).await {
        Ok(ops) => Ok(HttpResponse::Ok().json(ops)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Add work order material (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/manufacturing/work-orders/materials", tag = "Manufacturing",
    request_body = CreateWorkOrderMaterial,
    responses((status = 201, description = "Material added"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn add_work_order_material(
    _admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    payload: web::Json<CreateWorkOrderMaterial>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match mfg_service
        .add_work_order_material(payload.into_inner())
        .await
    {
        Ok(material) => Ok(HttpResponse::Created().json(material)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get work order materials
#[utoipa::path(
    get, path = "/api/v1/manufacturing/work-orders/{work_order_id}/materials", tag = "Manufacturing",
    params(("work_order_id" = i64, Path, description = "Work order ID")),
    responses((status = 200, description = "Materials for work order")),
    security(("bearer_auth" = []))
)]
pub async fn get_work_order_materials(
    _auth_user: AuthUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match mfg_service.get_work_order_materials(*path).await {
        Ok(materials) => Ok(HttpResponse::Ok().json(materials)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

// --- Bills of Materials ---

/// Create BOM (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/manufacturing/boms", tag = "Manufacturing",
    request_body = CreateBillOfMaterials,
    responses((status = 201, description = "BOM created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_bom(
    admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    payload: web::Json<CreateBillOfMaterials>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    match mfg_service.create_bom(create).await {
        Ok(bom) => Ok(HttpResponse::Created().json(bom)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get BOM by ID
#[utoipa::path(
    get, path = "/api/v1/manufacturing/boms/{id}", tag = "Manufacturing",
    params(("id" = i64, Path, description = "BOM ID")),
    responses((status = 200, description = "BOM found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_bom(
    auth_user: AuthUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match mfg_service.get_bom(*path, auth_user.0.tenant_id).await {
        Ok(bom) => Ok(HttpResponse::Ok().json(bom)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get BOMs by product
#[utoipa::path(
    get, path = "/api/v1/manufacturing/boms/product/{product_id}", tag = "Manufacturing",
    params(("product_id" = i64, Path, description = "Product ID")),
    responses((status = 200, description = "BOMs for product")),
    security(("bearer_auth" = []))
)]
pub async fn get_boms_by_product(
    _auth_user: AuthUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match mfg_service.get_boms_by_product(*path).await {
        Ok(boms) => Ok(HttpResponse::Ok().json(boms)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Add BOM line (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/manufacturing/boms/lines", tag = "Manufacturing",
    request_body = CreateBillOfMaterialsLine,
    responses((status = 201, description = "BOM line added"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn add_bom_line(
    _admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    payload: web::Json<CreateBillOfMaterialsLine>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match mfg_service.add_bom_line(payload.into_inner()).await {
        Ok(line) => Ok(HttpResponse::Created().json(line)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get BOM lines
#[utoipa::path(
    get, path = "/api/v1/manufacturing/boms/{bom_id}/lines", tag = "Manufacturing",
    params(("bom_id" = i64, Path, description = "BOM ID")),
    responses((status = 200, description = "BOM lines")),
    security(("bearer_auth" = []))
)]
pub async fn get_bom_lines(
    _auth_user: AuthUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match mfg_service.get_bom_lines(*path).await {
        Ok(lines) => Ok(HttpResponse::Ok().json(lines)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

// --- Routings ---

/// Create routing (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/manufacturing/routings", tag = "Manufacturing",
    request_body = CreateRouting,
    responses((status = 201, description = "Routing created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_routing(
    admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    payload: web::Json<CreateRouting>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    match mfg_service.create_routing(create).await {
        Ok(routing) => Ok(HttpResponse::Created().json(routing)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get routing by ID
#[utoipa::path(
    get, path = "/api/v1/manufacturing/routings/{id}", tag = "Manufacturing",
    params(("id" = i64, Path, description = "Routing ID")),
    responses((status = 200, description = "Routing found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_routing(
    auth_user: AuthUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match mfg_service.get_routing(*path, auth_user.0.tenant_id).await {
        Ok(routing) => Ok(HttpResponse::Ok().json(routing)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get routings by product
#[utoipa::path(
    get, path = "/api/v1/manufacturing/routings/product/{product_id}", tag = "Manufacturing",
    params(("product_id" = i64, Path, description = "Product ID")),
    responses((status = 200, description = "Routings for product")),
    security(("bearer_auth" = []))
)]
pub async fn get_routings_by_product(
    _auth_user: AuthUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match mfg_service.get_routings_by_product(*path).await {
        Ok(routings) => Ok(HttpResponse::Ok().json(routings)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Add routing operation (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/manufacturing/routings/operations", tag = "Manufacturing",
    request_body = CreateRoutingOperation,
    responses((status = 201, description = "Routing operation added"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn add_routing_operation(
    _admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    payload: web::Json<CreateRoutingOperation>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match mfg_service
        .add_routing_operation(payload.into_inner())
        .await
    {
        Ok(op) => Ok(HttpResponse::Created().json(op)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Calculate material requirements
#[utoipa::path(
    get, path = "/api/v1/manufacturing/material-requirements/{product_id}", tag = "Manufacturing",
    params(("product_id" = i64, Path, description = "Product ID"), ("quantity" = String, Query, description = "Quantity")),
    responses((status = 200, description = "Material requirements")),
    security(("bearer_auth" = []))
)]
pub async fn calculate_material_requirements(
    _auth_user: AuthUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
    query: web::Query<QuantityQuery>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let quantity: rust_decimal::Decimal = query.quantity.parse().map_err(|_| {
        let msg = i18n.t(locale.as_str(), "generic.validation_error");
        ApiError::Validation(msg)
    })?;
    match mfg_service
        .calculate_material_requirements(*path, quantity)
        .await
    {
        Ok(requirements) => Ok(HttpResponse::Ok().json(requirements)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Create inspection (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/manufacturing/inspections", tag = "Manufacturing",
    request_body = CreateInspection,
    responses((status = 201, description = "Inspection created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_inspection(
    _admin_user: AdminUser,
    qc_service: web::Data<QualityControlService>,
    payload: web::Json<CreateInspection>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let create = payload.into_inner();
    match qc_service.create_inspection(create).await {
        Ok(inspection) => Ok(HttpResponse::Created().json(inspection)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get all inspections for tenant
#[utoipa::path(
    get, path = "/api/v1/manufacturing/inspections", tag = "Manufacturing",
    responses((status = 200, description = "List of inspections")),
    security(("bearer_auth" = []))
)]
pub async fn get_inspections(
    auth_user: AuthUser,
    qc_service: web::Data<QualityControlService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match qc_service
        .get_inspections_by_tenant(auth_user.0.tenant_id)
        .await
    {
        Ok(inspections) => Ok(HttpResponse::Ok().json(inspections)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get inspection by ID
#[utoipa::path(
    get, path = "/api/v1/manufacturing/inspections/{id}", tag = "Manufacturing",
    params(("id" = i64, Path, description = "Inspection ID")),
    responses((status = 200, description = "Inspection found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_inspection(
    auth_user: AuthUser,
    qc_service: web::Data<QualityControlService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match qc_service
        .get_inspection(*path, auth_user.0.tenant_id)
        .await
    {
        Ok(inspection) => Ok(HttpResponse::Ok().json(inspection)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Update inspection (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/manufacturing/inspections/{id}", tag = "Manufacturing",
    params(("id" = i64, Path, description = "Inspection ID")),
    request_body = UpdateInspection,
    responses((status = 200, description = "Inspection updated"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn update_inspection(
    _admin_user: AdminUser,
    qc_service: web::Data<QualityControlService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateInspection>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match qc_service
        .update_inspection(*path, payload.into_inner())
        .await
    {
        Ok(inspection) => Ok(HttpResponse::Ok().json(inspection)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Delete inspection (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/manufacturing/inspections/{id}", tag = "Manufacturing",
    params(("id" = i64, Path, description = "Inspection ID")),
    responses((status = 200, description = "Inspection deleted", body = MessageResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn delete_inspection(
    admin_user: AdminUser,
    qc_service: web::Data<QualityControlService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match qc_service
        .delete_inspection(*path, admin_user.0.tenant_id, admin_user.0.user_id()?)
        .await
    {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "manufacturing.inspection.deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

// --- NCR ---

/// Create NCR (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/manufacturing/ncrs", tag = "Manufacturing",
    request_body = CreateNonConformanceReport,
    responses((status = 201, description = "NCR created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_ncr(
    _admin_user: AdminUser,
    qc_service: web::Data<QualityControlService>,
    payload: web::Json<CreateNonConformanceReport>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let create = payload.into_inner();
    match qc_service.create_ncr(create).await {
        Ok(ncr) => Ok(HttpResponse::Created().json(ncr)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get all NCRs for tenant
#[utoipa::path(
    get, path = "/api/v1/manufacturing/ncrs", tag = "Manufacturing",
    responses((status = 200, description = "List of NCRs")),
    security(("bearer_auth" = []))
)]
pub async fn get_ncrs(
    auth_user: AuthUser,
    qc_service: web::Data<QualityControlService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match qc_service.get_ncrs_by_tenant(auth_user.0.tenant_id).await {
        Ok(ncrs) => Ok(HttpResponse::Ok().json(ncrs)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get NCR by ID
#[utoipa::path(
    get, path = "/api/v1/manufacturing/ncrs/{id}", tag = "Manufacturing",
    params(("id" = i64, Path, description = "NCR ID")),
    responses((status = 200, description = "NCR found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_ncr(
    auth_user: AuthUser,
    qc_service: web::Data<QualityControlService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match qc_service.get_ncr(*path, auth_user.0.tenant_id).await {
        Ok(ncr) => Ok(HttpResponse::Ok().json(ncr)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Update NCR (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/manufacturing/ncrs/{id}", tag = "Manufacturing",
    params(("id" = i64, Path, description = "NCR ID")),
    request_body = UpdateNonConformanceReport,
    responses((status = 200, description = "NCR updated"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn update_ncr(
    _admin_user: AdminUser,
    qc_service: web::Data<QualityControlService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateNonConformanceReport>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match qc_service.update_ncr(*path, payload.into_inner()).await {
        Ok(ncr) => Ok(HttpResponse::Ok().json(ncr)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Delete NCR (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/manufacturing/ncrs/{id}", tag = "Manufacturing",
    params(("id" = i64, Path, description = "NCR ID")),
    responses((status = 200, description = "NCR deleted", body = MessageResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn delete_ncr(
    admin_user: AdminUser,
    qc_service: web::Data<QualityControlService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match qc_service
        .delete_ncr(*path, admin_user.0.tenant_id, admin_user.0.user_id()?)
        .await
    {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "manufacturing.ncr.deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

// --- Soft-delete / Restore / Destroy endpoints ---

/// Soft-delete a work order (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/manufacturing/work-orders/{id}", tag = "Manufacturing",
    params(("id" = i64, Path, description = "Work order ID")),
    responses((status = 200, description = "Work order soft-deleted", body = MessageResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn soft_delete_work_order(
    admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match mfg_service
        .soft_delete_work_order(*path, admin_user.0.tenant_id, admin_user.0.user_id()?)
        .await
    {
        Ok(()) => Ok(HttpResponse::Ok().json(MessageResponse {
            message: "Work order soft-deleted".to_string(),
        })),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted work order (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/manufacturing/work-orders/{id}/restore", tag = "Manufacturing",
    params(("id" = i64, Path, description = "Work order ID")),
    responses((status = 200, description = "Work order restored"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn restore_work_order(
    admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let work_order = mfg_service
        .restore_work_order(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(work_order))
}

/// List soft-deleted work orders (requires admin role)
#[utoipa::path(
    get, path = "/api/v1/manufacturing/work-orders/deleted", tag = "Manufacturing",
    responses((status = 200, description = "List of deleted work orders")),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_work_orders(
    admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
) -> ApiResult<HttpResponse> {
    let deleted = mfg_service
        .list_deleted_work_orders(admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(deleted))
}

/// Permanently destroy a work order (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/manufacturing/work-orders/{id}/destroy", tag = "Manufacturing",
    params(("id" = i64, Path, description = "Work order ID")),
    responses((status = 204, description = "Work order permanently deleted"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn destroy_work_order(
    admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    mfg_service
        .destroy_work_order(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Soft-delete a BOM (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/manufacturing/boms/{id}", tag = "Manufacturing",
    params(("id" = i64, Path, description = "BOM ID")),
    responses((status = 200, description = "BOM soft-deleted", body = MessageResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn soft_delete_bom(
    admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match mfg_service
        .soft_delete_bom(*path, admin_user.0.tenant_id, admin_user.0.user_id()?)
        .await
    {
        Ok(()) => Ok(HttpResponse::Ok().json(MessageResponse {
            message: "BOM soft-deleted".to_string(),
        })),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted BOM (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/manufacturing/boms/{id}/restore", tag = "Manufacturing",
    params(("id" = i64, Path, description = "BOM ID")),
    responses((status = 200, description = "BOM restored"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn restore_bom(
    admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let bom = mfg_service
        .restore_bom(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(bom))
}

/// List soft-deleted BOMs (requires admin role)
#[utoipa::path(
    get, path = "/api/v1/manufacturing/boms/deleted", tag = "Manufacturing",
    responses((status = 200, description = "List of deleted BOMs")),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_boms(
    admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
) -> ApiResult<HttpResponse> {
    let deleted = mfg_service
        .list_deleted_boms(admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(deleted))
}

/// Permanently destroy a BOM (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/manufacturing/boms/{id}/destroy", tag = "Manufacturing",
    params(("id" = i64, Path, description = "BOM ID")),
    responses((status = 204, description = "BOM permanently deleted"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn destroy_bom(
    admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    mfg_service
        .destroy_bom(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Soft-delete a routing (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/manufacturing/routings/{id}", tag = "Manufacturing",
    params(("id" = i64, Path, description = "Routing ID")),
    responses((status = 200, description = "Routing soft-deleted", body = MessageResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn soft_delete_routing(
    admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match mfg_service
        .soft_delete_routing(*path, admin_user.0.tenant_id, admin_user.0.user_id()?)
        .await
    {
        Ok(()) => Ok(HttpResponse::Ok().json(MessageResponse {
            message: "Routing soft-deleted".to_string(),
        })),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted routing (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/manufacturing/routings/{id}/restore", tag = "Manufacturing",
    params(("id" = i64, Path, description = "Routing ID")),
    responses((status = 200, description = "Routing restored"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn restore_routing(
    admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let routing = mfg_service
        .restore_routing(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(routing))
}

/// List soft-deleted routings (requires admin role)
#[utoipa::path(
    get, path = "/api/v1/manufacturing/routings/deleted", tag = "Manufacturing",
    responses((status = 200, description = "List of deleted routings")),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_routings(
    admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
) -> ApiResult<HttpResponse> {
    let deleted = mfg_service
        .list_deleted_routings(admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(deleted))
}

/// Permanently destroy a routing (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/manufacturing/routings/{id}/destroy", tag = "Manufacturing",
    params(("id" = i64, Path, description = "Routing ID")),
    responses((status = 204, description = "Routing permanently deleted"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn destroy_routing(
    admin_user: AdminUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    mfg_service
        .destroy_routing(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Restore a soft-deleted inspection (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/manufacturing/inspections/{id}/restore", tag = "Manufacturing",
    params(("id" = i64, Path, description = "Inspection ID")),
    responses((status = 200, description = "Inspection restored"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn restore_inspection(
    admin_user: AdminUser,
    qc_service: web::Data<QualityControlService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let inspection = qc_service
        .restore_inspection(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(inspection))
}

/// List soft-deleted inspections (requires admin role)
#[utoipa::path(
    get, path = "/api/v1/manufacturing/inspections/deleted", tag = "Manufacturing",
    responses((status = 200, description = "List of deleted inspections")),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_inspections(
    admin_user: AdminUser,
    qc_service: web::Data<QualityControlService>,
) -> ApiResult<HttpResponse> {
    let deleted = qc_service
        .list_deleted_inspections(admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(deleted))
}

/// Permanently destroy an inspection (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/manufacturing/inspections/{id}/destroy", tag = "Manufacturing",
    params(("id" = i64, Path, description = "Inspection ID")),
    responses((status = 204, description = "Inspection permanently deleted"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn destroy_inspection(
    admin_user: AdminUser,
    qc_service: web::Data<QualityControlService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    qc_service
        .destroy_inspection(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Restore a soft-deleted NCR (requires admin role)
#[utoipa::path(
    put, path = "/api/v1/manufacturing/ncrs/{id}/restore", tag = "Manufacturing",
    params(("id" = i64, Path, description = "NCR ID")),
    responses((status = 200, description = "NCR restored"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn restore_ncr(
    admin_user: AdminUser,
    qc_service: web::Data<QualityControlService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let ncr = qc_service
        .restore_ncr(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(ncr))
}

/// List soft-deleted NCRs (requires admin role)
#[utoipa::path(
    get, path = "/api/v1/manufacturing/ncrs/deleted", tag = "Manufacturing",
    responses((status = 200, description = "List of deleted NCRs")),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_ncrs(
    admin_user: AdminUser,
    qc_service: web::Data<QualityControlService>,
) -> ApiResult<HttpResponse> {
    let deleted = qc_service.list_deleted_ncrs(admin_user.0.tenant_id).await?;
    Ok(HttpResponse::Ok().json(deleted))
}

/// Permanently destroy an NCR (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/manufacturing/ncrs/{id}/destroy", tag = "Manufacturing",
    params(("id" = i64, Path, description = "NCR ID")),
    responses((status = 204, description = "NCR permanently deleted"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn destroy_ncr(
    admin_user: AdminUser,
    qc_service: web::Data<QualityControlService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    qc_service
        .destroy_ncr(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct UpdateWorkOrderStatusRequest {
    pub status: WorkOrderStatus,
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct QuantityQuery {
    pub quantity: String,
}

/// Configure manufacturing routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/manufacturing/work-orders")
            .route(web::get().to(get_work_orders))
            .route(web::post().to(create_work_order)),
    )
    .service(
        web::resource("/v1/manufacturing/work-orders/deleted")
            .route(web::get().to(list_deleted_work_orders)),
    )
    .service(
        web::resource("/v1/manufacturing/work-orders/{id}")
            .route(web::get().to(get_work_order))
            .route(web::delete().to(soft_delete_work_order)),
    )
    .service(
        web::resource("/v1/manufacturing/work-orders/{id}/status")
            .route(web::put().to(update_work_order_status)),
    )
    .service(
        web::resource("/v1/manufacturing/work-orders/{id}/restore")
            .route(web::put().to(restore_work_order)),
    )
    .service(
        web::resource("/v1/manufacturing/work-orders/{id}/destroy")
            .route(web::delete().to(destroy_work_order)),
    )
    .service(
        web::resource("/v1/manufacturing/work-orders/{work_order_id}/operations")
            .route(web::get().to(get_work_order_operations)),
    )
    .service(
        web::resource("/v1/manufacturing/work-orders/{work_order_id}/materials")
            .route(web::get().to(get_work_order_materials)),
    )
    .service(
        web::resource("/v1/manufacturing/work-orders/operations")
            .route(web::post().to(add_work_order_operation)),
    )
    .service(
        web::resource("/v1/manufacturing/work-orders/materials")
            .route(web::post().to(add_work_order_material)),
    )
    .service(web::resource("/v1/manufacturing/boms").route(web::post().to(create_bom)))
    .service(
        web::resource("/v1/manufacturing/boms/deleted").route(web::get().to(list_deleted_boms)),
    )
    .service(
        web::resource("/v1/manufacturing/boms/{id}")
            .route(web::get().to(get_bom))
            .route(web::delete().to(soft_delete_bom)),
    )
    .service(web::resource("/v1/manufacturing/boms/{id}/restore").route(web::put().to(restore_bom)))
    .service(
        web::resource("/v1/manufacturing/boms/{id}/destroy").route(web::delete().to(destroy_bom)),
    )
    .service(
        web::resource("/v1/manufacturing/boms/{bom_id}/lines").route(web::get().to(get_bom_lines)),
    )
    .service(web::resource("/v1/manufacturing/boms/lines").route(web::post().to(add_bom_line)))
    .service(
        web::resource("/v1/manufacturing/boms/product/{product_id}")
            .route(web::get().to(get_boms_by_product)),
    )
    .service(web::resource("/v1/manufacturing/routings").route(web::post().to(create_routing)))
    .service(
        web::resource("/v1/manufacturing/routings/deleted")
            .route(web::get().to(list_deleted_routings)),
    )
    .service(
        web::resource("/v1/manufacturing/routings/{id}")
            .route(web::get().to(get_routing))
            .route(web::delete().to(soft_delete_routing)),
    )
    .service(
        web::resource("/v1/manufacturing/routings/{id}/restore")
            .route(web::put().to(restore_routing)),
    )
    .service(
        web::resource("/v1/manufacturing/routings/{id}/destroy")
            .route(web::delete().to(destroy_routing)),
    )
    .service(
        web::resource("/v1/manufacturing/routings/product/{product_id}")
            .route(web::get().to(get_routings_by_product)),
    )
    .service(
        web::resource("/v1/manufacturing/routings/operations")
            .route(web::post().to(add_routing_operation)),
    )
    .service(
        web::resource("/v1/manufacturing/material-requirements/{product_id}")
            .route(web::get().to(calculate_material_requirements)),
    )
    // Quality Control - Inspections
    .service(
        web::resource("/v1/manufacturing/inspections")
            .route(web::get().to(get_inspections))
            .route(web::post().to(create_inspection)),
    )
    .service(
        web::resource("/v1/manufacturing/inspections/deleted")
            .route(web::get().to(list_deleted_inspections)),
    )
    .service(
        web::resource("/v1/manufacturing/inspections/{id}")
            .route(web::get().to(get_inspection))
            .route(web::put().to(update_inspection))
            .route(web::delete().to(delete_inspection)),
    )
    .service(
        web::resource("/v1/manufacturing/inspections/{id}/restore")
            .route(web::put().to(restore_inspection)),
    )
    .service(
        web::resource("/v1/manufacturing/inspections/{id}/destroy")
            .route(web::delete().to(destroy_inspection)),
    )
    // Quality Control - NCRs
    .service(
        web::resource("/v1/manufacturing/ncrs")
            .route(web::get().to(get_ncrs))
            .route(web::post().to(create_ncr)),
    )
    .service(
        web::resource("/v1/manufacturing/ncrs/deleted").route(web::get().to(list_deleted_ncrs)),
    )
    .service(
        web::resource("/v1/manufacturing/ncrs/{id}")
            .route(web::get().to(get_ncr))
            .route(web::put().to(update_ncr))
            .route(web::delete().to(delete_ncr)),
    )
    .service(web::resource("/v1/manufacturing/ncrs/{id}/restore").route(web::put().to(restore_ncr)))
    .service(
        web::resource("/v1/manufacturing/ncrs/{id}/destroy").route(web::delete().to(destroy_ncr)),
    );
}
