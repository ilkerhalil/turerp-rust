//! Manufacturing API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::common::pagination::PaginationParams;
use crate::domain::manufacturing::model::{
    CreateBillOfMaterials, CreateBillOfMaterialsLine, CreateInspection, CreateNonConformanceReport,
    CreateRouting, CreateRoutingOperation, CreateWorkOrder, CreateWorkOrderMaterial,
    CreateWorkOrderOperation, UpdateInspection, UpdateNonConformanceReport, WorkOrderStatus,
};
use crate::domain::manufacturing::service::{ManufacturingService, QualityControlService};
use crate::error::ApiResult;
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
) -> ApiResult<HttpResponse> {
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    let order = mfg_service.create_work_order(create).await?;
    Ok(HttpResponse::Created().json(order))
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
) -> ApiResult<HttpResponse> {
    let pagination = query.into_inner();
    pagination
        .validate()
        .map_err(crate::error::ApiError::Validation)?;
    let result = mfg_service
        .get_work_orders_by_tenant_paginated(
            auth_user.0.tenant_id,
            pagination.page,
            pagination.per_page,
        )
        .await?;
    Ok(HttpResponse::Ok().json(result))
}

/// Get work order by ID
#[utoipa::path(
    get, path = "/api/v1/manufacturing/work-orders/{id}", tag = "Manufacturing",
    params(("id" = i64, Path, description = "Work order ID")),
    responses((status = 200, description = "Work order found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_work_order(
    _auth_user: AuthUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let order = mfg_service.get_work_order(*path).await?;
    Ok(HttpResponse::Ok().json(order))
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
) -> ApiResult<HttpResponse> {
    let order = mfg_service
        .update_work_order_status(*path, payload.into_inner().status)
        .await?;
    Ok(HttpResponse::Ok().json(order))
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
) -> ApiResult<HttpResponse> {
    let op = mfg_service
        .add_work_order_operation(payload.into_inner())
        .await?;
    Ok(HttpResponse::Created().json(op))
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
) -> ApiResult<HttpResponse> {
    let ops = mfg_service.get_work_order_operations(*path).await?;
    Ok(HttpResponse::Ok().json(ops))
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
) -> ApiResult<HttpResponse> {
    let material = mfg_service
        .add_work_order_material(payload.into_inner())
        .await?;
    Ok(HttpResponse::Created().json(material))
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
) -> ApiResult<HttpResponse> {
    let materials = mfg_service.get_work_order_materials(*path).await?;
    Ok(HttpResponse::Ok().json(materials))
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
) -> ApiResult<HttpResponse> {
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    let bom = mfg_service.create_bom(create).await?;
    Ok(HttpResponse::Created().json(bom))
}

/// Get BOM by ID
#[utoipa::path(
    get, path = "/api/v1/manufacturing/boms/{id}", tag = "Manufacturing",
    params(("id" = i64, Path, description = "BOM ID")),
    responses((status = 200, description = "BOM found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_bom(
    _auth_user: AuthUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let bom = mfg_service.get_bom(*path).await?;
    Ok(HttpResponse::Ok().json(bom))
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
) -> ApiResult<HttpResponse> {
    let boms = mfg_service.get_boms_by_product(*path).await?;
    Ok(HttpResponse::Ok().json(boms))
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
) -> ApiResult<HttpResponse> {
    let line = mfg_service.add_bom_line(payload.into_inner()).await?;
    Ok(HttpResponse::Created().json(line))
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
) -> ApiResult<HttpResponse> {
    let lines = mfg_service.get_bom_lines(*path).await?;
    Ok(HttpResponse::Ok().json(lines))
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
) -> ApiResult<HttpResponse> {
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    let routing = mfg_service.create_routing(create).await?;
    Ok(HttpResponse::Created().json(routing))
}

/// Get routing by ID
#[utoipa::path(
    get, path = "/api/v1/manufacturing/routings/{id}", tag = "Manufacturing",
    params(("id" = i64, Path, description = "Routing ID")),
    responses((status = 200, description = "Routing found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_routing(
    _auth_user: AuthUser,
    mfg_service: web::Data<ManufacturingService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let routing = mfg_service.get_routing(*path).await?;
    Ok(HttpResponse::Ok().json(routing))
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
) -> ApiResult<HttpResponse> {
    let routings = mfg_service.get_routings_by_product(*path).await?;
    Ok(HttpResponse::Ok().json(routings))
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
) -> ApiResult<HttpResponse> {
    let op = mfg_service
        .add_routing_operation(payload.into_inner())
        .await?;
    Ok(HttpResponse::Created().json(op))
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
) -> ApiResult<HttpResponse> {
    let quantity: rust_decimal::Decimal = query
        .quantity
        .parse()
        .map_err(|_| crate::error::ApiError::Validation("Invalid quantity".into()))?;
    let requirements = mfg_service
        .calculate_material_requirements(*path, quantity)
        .await?;
    Ok(HttpResponse::Ok().json(requirements))
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
) -> ApiResult<HttpResponse> {
    let create = payload.into_inner();
    let inspection = qc_service.create_inspection(create).await?;
    Ok(HttpResponse::Created().json(inspection))
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
) -> ApiResult<HttpResponse> {
    let inspections = qc_service
        .get_inspections_by_tenant(auth_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(inspections))
}

/// Get inspection by ID
#[utoipa::path(
    get, path = "/api/v1/manufacturing/inspections/{id}", tag = "Manufacturing",
    params(("id" = i64, Path, description = "Inspection ID")),
    responses((status = 200, description = "Inspection found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_inspection(
    _auth_user: AuthUser,
    qc_service: web::Data<QualityControlService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let inspection = qc_service.get_inspection(*path).await?;
    Ok(HttpResponse::Ok().json(inspection))
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
) -> ApiResult<HttpResponse> {
    let inspection = qc_service
        .update_inspection(*path, payload.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(inspection))
}

/// Delete inspection (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/manufacturing/inspections/{id}", tag = "Manufacturing",
    params(("id" = i64, Path, description = "Inspection ID")),
    responses((status = 204, description = "Inspection deleted"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn delete_inspection(
    _admin_user: AdminUser,
    qc_service: web::Data<QualityControlService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    qc_service.delete_inspection(*path).await?;
    Ok(HttpResponse::NoContent().finish())
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
) -> ApiResult<HttpResponse> {
    let create = payload.into_inner();
    let ncr = qc_service.create_ncr(create).await?;
    Ok(HttpResponse::Created().json(ncr))
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
) -> ApiResult<HttpResponse> {
    let ncrs = qc_service.get_ncrs_by_tenant(auth_user.0.tenant_id).await?;
    Ok(HttpResponse::Ok().json(ncrs))
}

/// Get NCR by ID
#[utoipa::path(
    get, path = "/api/v1/manufacturing/ncrs/{id}", tag = "Manufacturing",
    params(("id" = i64, Path, description = "NCR ID")),
    responses((status = 200, description = "NCR found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_ncr(
    _auth_user: AuthUser,
    qc_service: web::Data<QualityControlService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let ncr = qc_service.get_ncr(*path).await?;
    Ok(HttpResponse::Ok().json(ncr))
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
) -> ApiResult<HttpResponse> {
    let ncr = qc_service.update_ncr(*path, payload.into_inner()).await?;
    Ok(HttpResponse::Ok().json(ncr))
}

/// Delete NCR (requires admin role)
#[utoipa::path(
    delete, path = "/api/v1/manufacturing/ncrs/{id}", tag = "Manufacturing",
    params(("id" = i64, Path, description = "NCR ID")),
    responses((status = 204, description = "NCR deleted"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn delete_ncr(
    _admin_user: AdminUser,
    qc_service: web::Data<QualityControlService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    qc_service.delete_ncr(*path).await?;
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
        web::resource("/v1/manufacturing/work-orders/{id}").route(web::get().to(get_work_order)),
    )
    .service(
        web::resource("/v1/manufacturing/work-orders/{id}/status")
            .route(web::put().to(update_work_order_status)),
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
    .service(web::resource("/v1/manufacturing/boms/{id}").route(web::get().to(get_bom)))
    .service(
        web::resource("/v1/manufacturing/boms/{bom_id}/lines").route(web::get().to(get_bom_lines)),
    )
    .service(web::resource("/v1/manufacturing/boms/lines").route(web::post().to(add_bom_line)))
    .service(
        web::resource("/v1/manufacturing/boms/product/{product_id}")
            .route(web::get().to(get_boms_by_product)),
    )
    .service(web::resource("/v1/manufacturing/routings").route(web::post().to(create_routing)))
    .service(web::resource("/v1/manufacturing/routings/{id}").route(web::get().to(get_routing)))
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
        web::resource("/v1/manufacturing/inspections/{id}")
            .route(web::get().to(get_inspection))
            .route(web::put().to(update_inspection))
            .route(web::delete().to(delete_inspection)),
    )
    // Quality Control - NCRs
    .service(
        web::resource("/v1/manufacturing/ncrs")
            .route(web::get().to(get_ncrs))
            .route(web::post().to(create_ncr)),
    )
    .service(
        web::resource("/v1/manufacturing/ncrs/{id}")
            .route(web::get().to(get_ncr))
            .route(web::put().to(update_ncr))
            .route(web::delete().to(delete_ncr)),
    );
}
