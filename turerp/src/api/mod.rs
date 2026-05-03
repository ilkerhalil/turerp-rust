//! API layer

pub mod v1;

// Legacy modules (deprecated, will be removed in v2)
pub mod auth;
pub mod users;

// Explicit re-exports to avoid ambiguity
pub use auth::configure as auth_configure;
pub use users::configure as users_configure;

// V1 re-exports
pub use v1::accounting_configure as v1_accounting_configure;
pub use v1::assets_configure as v1_assets_configure;
pub use v1::audit_configure as v1_audit_configure;
pub use v1::auth_configure as v1_auth_configure;
pub use v1::cari_configure as v1_cari_configure;
pub use v1::crm_configure as v1_crm_configure;
pub use v1::feature_flags_configure as v1_feature_flags_configure;
pub use v1::hr_configure as v1_hr_configure;
pub use v1::invoice_configure as v1_invoice_configure;
pub use v1::manufacturing_configure as v1_manufacturing_configure;
pub use v1::product_variants_configure as v1_product_variants_configure;
pub use v1::project_configure as v1_project_configure;
pub use v1::purchase_requests_configure as v1_purchase_requests_configure;
pub use v1::sales_configure as v1_sales_configure;
pub use v1::settings_configure as v1_settings_configure;
pub use v1::stock_configure as v1_stock_configure;
pub use v1::tenant_configure as v1_tenant_configure;
pub use v1::users_configure as v1_users_configure;

use crate::common::MessageResponse;
use crate::domain::auth::{LoginRequest, LoginResponse, RefreshTokenRequest, RegisterRequest};
use crate::domain::feature::{
    CreateFeatureFlag, FeatureFlagResponse, FeatureFlagStatus, UpdateFeatureFlag,
};
use crate::domain::user::{CreateUser, UpdateUser, UserResponse};
use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa::Modify;
use utoipa::OpenApi;

/// OpenAPI specification for the Turerp ERP API
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Turerp ERP API",
        description = "Multi-tenant SaaS ERP system API\n\n## Authentication\n\nAll endpoints except `/api/v1/auth/login`, `/api/v1/auth/register`, and `/api/v1/auth/refresh` require JWT Bearer token authentication.\n\n## Rate Limiting\n\nAuthentication endpoints are rate limited to 10 requests per minute per IP address with a burst of 3 requests.\n\n## Localization\n\nAll endpoints support per-request localization via the `Accept-Language` header. Supported languages: `en` (default), `tr`.\n\n## API Versioning\n\n- `/api/v1/` - Current stable API (recommended)\n- `/api/auth/`, `/api/users/` - Legacy routes (deprecated, will be removed in v2)",
        version = "1.0.0",
        contact(
            name = "Turerp Team",
            email = "info@turerp.com"
        )
    ),
    paths(
        // Auth
        crate::api::v1::auth::register,
        crate::api::v1::auth::login,
        crate::api::v1::auth::refresh_token,
        crate::api::v1::auth::me,
        // Users
        crate::api::v1::users::create_user,
        crate::api::v1::users::get_user,
        crate::api::v1::users::get_users,
        crate::api::v1::users::update_user,
        crate::api::v1::users::delete_user,
        // Tenant
        crate::api::v1::tenant::create_tenant,
        crate::api::v1::tenant::get_tenants,
        crate::api::v1::tenant::get_tenant,
        crate::api::v1::tenant::update_tenant,
        crate::api::v1::tenant::delete_tenant,
        crate::api::v1::tenant::create_tenant_config,
        crate::api::v1::tenant::get_tenant_configs,
        crate::api::v1::tenant::get_tenant_config,
        crate::api::v1::tenant::update_tenant_config,
        crate::api::v1::tenant::delete_tenant_config,
        // Cari
        crate::api::v1::cari::create_cari,
        crate::api::v1::cari::get_cari,
        crate::api::v1::cari::get_all_cari,
        crate::api::v1::cari::get_cari_by_type,
        crate::api::v1::cari::search_cari,
        crate::api::v1::cari::update_cari,
        crate::api::v1::cari::delete_cari,
        // Stock
        crate::api::v1::stock::create_warehouse,
        crate::api::v1::stock::get_warehouses,
        crate::api::v1::stock::get_warehouse,
        crate::api::v1::stock::update_warehouse,
        crate::api::v1::stock::delete_warehouse,
        crate::api::v1::stock::create_stock_movement,
        crate::api::v1::stock::get_stock_movements_by_product,
        crate::api::v1::stock::get_stock_movements_by_warehouse,
        crate::api::v1::stock::get_stock_by_product,
        crate::api::v1::stock::get_stock_by_warehouse,
        crate::api::v1::stock::get_stock_summary,
        // Invoice
        crate::api::v1::invoice::create_invoice,
        crate::api::v1::invoice::get_invoice,
        crate::api::v1::invoice::get_invoices,
        crate::api::v1::invoice::get_invoices_by_status,
        crate::api::v1::invoice::get_outstanding_invoices,
        crate::api::v1::invoice::get_overdue_invoices,
        crate::api::v1::invoice::update_invoice_status,
        crate::api::v1::invoice::delete_invoice,
        crate::api::v1::invoice::create_payment,
        crate::api::v1::invoice::get_payments_by_invoice,
        // Sales
        crate::api::v1::sales::create_sales_order,
        crate::api::v1::sales::get_sales_orders,
        crate::api::v1::sales::get_sales_order,
        crate::api::v1::sales::get_sales_orders_by_status,
        crate::api::v1::sales::update_sales_order_status,
        crate::api::v1::sales::delete_sales_order,
        crate::api::v1::sales::create_quotation,
        crate::api::v1::sales::get_quotations,
        crate::api::v1::sales::get_quotation,
        crate::api::v1::sales::get_quotations_by_status,
        crate::api::v1::sales::update_quotation_status,
        crate::api::v1::sales::convert_quotation_to_order,
        crate::api::v1::sales::delete_quotation,
        // Purchase Requests
        crate::api::v1::purchase_requests::create_request,
        crate::api::v1::purchase_requests::get_requests,
        crate::api::v1::purchase_requests::get_request,
        crate::api::v1::purchase_requests::update_request,
        crate::api::v1::purchase_requests::submit_request,
        crate::api::v1::purchase_requests::approve_request,
        crate::api::v1::purchase_requests::reject_request,
        crate::api::v1::purchase_requests::delete_request,
        // HR
        crate::api::v1::hr::create_employee,
        crate::api::v1::hr::get_employees,
        crate::api::v1::hr::get_employee,
        crate::api::v1::hr::update_employee_status,
        crate::api::v1::hr::terminate_employee,
        crate::api::v1::hr::record_attendance,
        crate::api::v1::hr::get_attendance_by_employee,
        crate::api::v1::hr::create_leave_request,
        crate::api::v1::hr::get_leave_requests_by_employee,
        crate::api::v1::hr::approve_leave_request,
        crate::api::v1::hr::reject_leave_request,
        crate::api::v1::hr::get_leave_types,
        crate::api::v1::hr::calculate_payroll,
        crate::api::v1::hr::get_payroll_by_employee,
        crate::api::v1::hr::mark_payroll_paid,
        // Accounting
        crate::api::v1::accounting::create_account,
        crate::api::v1::accounting::get_accounts,
        crate::api::v1::accounting::get_accounts_by_type,
        crate::api::v1::accounting::get_account,
        crate::api::v1::accounting::create_journal_entry,
        crate::api::v1::accounting::get_journal_entries,
        crate::api::v1::accounting::get_journal_entry,
        crate::api::v1::accounting::post_journal_entry,
        crate::api::v1::accounting::void_journal_entry,
        crate::api::v1::accounting::generate_trial_balance,
        // Assets
        crate::api::v1::assets::create_asset,
        crate::api::v1::assets::get_assets,
        crate::api::v1::assets::get_asset,
        crate::api::v1::assets::get_assets_by_status,
        crate::api::v1::assets::update_asset,
        crate::api::v1::assets::update_asset_status,
        crate::api::v1::assets::calculate_depreciation,
        crate::api::v1::assets::record_depreciation,
        crate::api::v1::assets::dispose_asset,
        crate::api::v1::assets::write_off_asset,
        crate::api::v1::assets::start_maintenance,
        crate::api::v1::assets::end_maintenance,
        crate::api::v1::assets::delete_asset,
        crate::api::v1::assets::create_maintenance_record,
        crate::api::v1::assets::get_maintenance_records,
        // Project
        crate::api::v1::project::create_project,
        crate::api::v1::project::get_projects,
        crate::api::v1::project::get_project,
        crate::api::v1::project::update_project_status,
        crate::api::v1::project::create_wbs_item,
        crate::api::v1::project::get_wbs_by_project,
        crate::api::v1::project::update_wbs_progress,
        crate::api::v1::project::create_project_cost,
        crate::api::v1::project::get_project_costs,
        crate::api::v1::project::get_profitability,
        // Manufacturing
        crate::api::v1::manufacturing::create_work_order,
        crate::api::v1::manufacturing::get_work_orders,
        crate::api::v1::manufacturing::get_work_order,
        crate::api::v1::manufacturing::update_work_order_status,
        crate::api::v1::manufacturing::add_work_order_operation,
        crate::api::v1::manufacturing::get_work_order_operations,
        crate::api::v1::manufacturing::add_work_order_material,
        crate::api::v1::manufacturing::get_work_order_materials,
        crate::api::v1::manufacturing::create_bom,
        crate::api::v1::manufacturing::get_bom,
        crate::api::v1::manufacturing::get_boms_by_product,
        crate::api::v1::manufacturing::add_bom_line,
        crate::api::v1::manufacturing::get_bom_lines,
        crate::api::v1::manufacturing::create_routing,
        crate::api::v1::manufacturing::get_routing,
        crate::api::v1::manufacturing::get_routings_by_product,
        crate::api::v1::manufacturing::add_routing_operation,
        crate::api::v1::manufacturing::calculate_material_requirements,
        crate::api::v1::manufacturing::create_inspection,
        crate::api::v1::manufacturing::get_inspections,
        crate::api::v1::manufacturing::get_inspection,
        crate::api::v1::manufacturing::update_inspection,
        crate::api::v1::manufacturing::delete_inspection,
        crate::api::v1::manufacturing::create_ncr,
        crate::api::v1::manufacturing::get_ncrs,
        crate::api::v1::manufacturing::get_ncr,
        crate::api::v1::manufacturing::update_ncr,
        crate::api::v1::manufacturing::delete_ncr,
        // CRM
        crate::api::v1::crm::create_lead,
        crate::api::v1::crm::get_leads,
        crate::api::v1::crm::get_lead,
        crate::api::v1::crm::get_leads_by_status,
        crate::api::v1::crm::update_lead_status,
        crate::api::v1::crm::convert_lead,
        crate::api::v1::crm::create_opportunity,
        crate::api::v1::crm::get_opportunities,
        crate::api::v1::crm::get_opportunity,
        crate::api::v1::crm::get_opportunities_by_status,
        crate::api::v1::crm::update_opportunity_status,
        crate::api::v1::crm::get_pipeline_value,
        crate::api::v1::crm::create_campaign,
        crate::api::v1::crm::get_campaigns,
        crate::api::v1::crm::get_campaign,
        crate::api::v1::crm::get_campaigns_by_status,
        crate::api::v1::crm::update_campaign_status,
        crate::api::v1::crm::create_ticket,
        crate::api::v1::crm::get_tickets,
        crate::api::v1::crm::get_ticket,
        crate::api::v1::crm::get_tickets_by_status,
        crate::api::v1::crm::update_ticket_status,
        crate::api::v1::crm::resolve_ticket,
        crate::api::v1::crm::get_open_tickets_count,
        // Products
        crate::api::v1::product_variants::get_products,
        crate::api::v1::product_variants::get_categories,
        crate::api::v1::product_variants::create_variant,
        crate::api::v1::product_variants::get_variants_by_product,
        crate::api::v1::product_variants::get_variant,
        crate::api::v1::product_variants::update_variant,
        crate::api::v1::product_variants::delete_variant,
        // Feature Flags
        crate::api::v1::feature_flags::create_flag,
        crate::api::v1::feature_flags::get_flags,
        crate::api::v1::feature_flags::get_flag_by_id,
        crate::api::v1::feature_flags::update_flag,
        crate::api::v1::feature_flags::delete_flag,
        crate::api::v1::feature_flags::enable_flag,
        crate::api::v1::feature_flags::disable_flag,
        crate::api::v1::feature_flags::check_feature,
        // Audit
        crate::api::v1::audit::get_audit_logs,
    ),
    components(
        schemas(
            // Core
            LoginRequest,
            LoginResponse,
            RefreshTokenRequest,
            RegisterRequest,
            CreateUser,
            UpdateUser,
            UserResponse,
            MessageResponse,
            crate::error::ErrorResponse,
            crate::common::PaginationParams,
            // Feature flags
            CreateFeatureFlag,
            UpdateFeatureFlag,
            FeatureFlagResponse,
            FeatureFlagStatus,
            crate::api::v1::feature_flags::FeatureStatusResponse,
        )
    ),
    security(
        ("bearer_auth" = [])
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Auth", description = "Authentication endpoints (login, register, token refresh)"),
        (name = "Users", description = "User management endpoints (CRUD operations)"),
        (name = "Tenant", description = "Tenant management and configuration"),
        (name = "Cari", description = "Customer / Vendor accounts"),
        (name = "Stock", description = "Warehouses, stock movements and inventory"),
        (name = "Invoice", description = "Invoices and payments"),
        (name = "Sales", description = "Sales orders and quotations"),
        (name = "Purchase Requests", description = "Purchase requests and approval workflow"),
        (name = "HR", description = "Employees, attendance, leave and payroll"),
        (name = "Accounting", description = "Chart of accounts and journal entries"),
        (name = "Assets", description = "Fixed assets, depreciation and maintenance"),
        (name = "Project", description = "Projects, WBS and costs"),
        (name = "Manufacturing", description = "Work orders, BOM, routing and quality control"),
        (name = "CRM", description = "Leads, opportunities, campaigns and tickets"),
        (name = "Products", description = "Product catalog, categories and variants"),
        (name = "Feature Flags", description = "Feature flag management endpoints"),
        (name = "Audit", description = "Audit log retrieval")
    )
)]
pub struct ApiDoc;

/// Security scheme addon for OpenAPI
pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
            );
        }
    }
}
