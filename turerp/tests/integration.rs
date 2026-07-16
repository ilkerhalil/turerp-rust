//! Consolidated integration test binary.
//!
//! All `tests/*.rs` files are declared as submodules here so they compile and
//! link into a single binary. This avoids re-linking the heavy dependency tree
//! 80 times (one per file) and dramatically cuts total test-compile time.
//!
//! `#[macro_use]` on `mod common;` makes the `#[macro_export]` macros defined
//! in `tests/common/mod.rs` available to every submodule below without a
//! per-file `use` import.

#![allow(clippy::module_inception)]

#[macro_use]
mod common;

mod accounting_crud_test;
mod api_integration_test;
mod api_key_crud_test;
mod archive_crud_test;
mod assets_crud_test;
mod audit_crud_test;
mod audit_dlq_test;
mod audit_tiered_backpressure_test;
mod audit_unauth_path_test;
mod audit_writer_panic_recovery_test;
mod bank_account_test;
mod bank_reconciliation_test;
mod bank_transaction_test;
mod barcode_crud_test;
mod cari_crud_test;
mod chart_of_accounts_crud_test;
mod company_crud_test;
mod config_validation_test;
mod cors_env_loading_test;
mod cost_center_allocation_test;
mod cost_center_crud_test;
mod crm_crud_test;
mod cross_tenant_register_test;
mod currency_crud_test;
mod custom_field_crud_test;
mod customer_portal_test;
mod dashboard_integration_test;
mod db_pool_max_conns_env_test;
mod document_crud_test;
mod earchive_crud_test;
mod edefter_crud_test;
mod efatura_crud_test;
mod feature_flag_crud_test;
mod files_integration_test;
mod forecasting_crud_test;
mod health_check_test;
mod health_ready_scheduler_probe_test;
mod hr_crud_test;
mod inter_company_api_test;
mod inter_company_repository_test;
mod invoice_crud_test;
mod ip_whitelist_crud_test;
mod job_crud_test;
mod job_executor_panic_backoff_preempts_shutdown_test;
mod job_executor_shutdown_test;
mod job_service_cron_runs_test;
mod ldap_crud_test;
mod manufacturing_crud_test;
mod mfa_crud_test;
mod middleware_gate_test;
mod migration_inventory_test;
mod negative_duration_cleanup_test;
mod notification_crud_test;
mod observability_test;
mod p0_cross_module_test;
mod performance_test;
mod product_crud_test;
mod project_crud_test;
mod purchase_crud_test;
mod quality_control_inspection_test;
mod quality_control_ncr_test;
mod rate_limit_defaults_test;
mod rate_limit_env_loading_test;
mod sales_crud_test;
mod security_test;
mod settings_crud_test;
mod shift_crud_test;
mod soft_delete_test;
mod stock_crud_test;
mod subscription_auth_test;
mod subscription_billing_test;
mod subscription_plan_test;
mod tax_crud_test;
mod tenant_crud_test;
mod user_crud_test;
mod vendor_portal_test;
mod webhook_crud_test;
mod workflow_auth_test;
mod workflow_crud_test;
mod workflow_instance_test;
mod workflow_template_test;
