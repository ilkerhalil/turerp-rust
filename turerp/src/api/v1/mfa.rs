//! MFA API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::domain::mfa::model::{DisableMfaRequest, VerifyMfaRequest, VerifyTotpRequest};
use crate::domain::mfa::service::MfaService;
use crate::domain::user::service::UserService;
use crate::error::ApiError;
use crate::middleware::AuthUser;

/// Start MFA setup endpoint (requires authentication)
#[utoipa::path(
    post,
    path = "/api/v1/auth/mfa/setup",
    tag = "Auth",
    responses(
        (status = 200, description = "MFA setup initiated", body = MfaSetupResponse),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
)]
pub async fn mfa_setup(
    auth_user: AuthUser,
    mfa_service: web::Data<MfaService>,
    user_service: web::Data<UserService>,
) -> Result<HttpResponse, ApiError> {
    let user_id: i64 = auth_user.0.user_id()?;
    let tenant_id = auth_user.0.tenant_id;

    let user = user_service.get_user(user_id, tenant_id).await?;

    let setup = mfa_service
        .setup_mfa(user_id, tenant_id, &user.email, "Turerp")
        .await?;

    Ok(HttpResponse::Ok().json(setup))
}

/// Verify MFA setup code endpoint (requires authentication)
#[utoipa::path(
    post,
    path = "/api/v1/auth/mfa/verify-setup",
    tag = "Auth",
    request_body = VerifyTotpRequest,
    responses(
        (status = 200, description = "MFA enabled successfully", body = MfaStatusResponse),
        (status = 400, description = "Invalid setup code"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
)]
pub async fn mfa_verify_setup(
    auth_user: AuthUser,
    mfa_service: web::Data<MfaService>,
    body: web::Json<VerifyTotpRequest>,
) -> Result<HttpResponse, ApiError> {
    let user_id: i64 = auth_user.0.user_id()?;
    let tenant_id = auth_user.0.tenant_id;

    let status = mfa_service
        .verify_setup(user_id, tenant_id, &body.code)
        .await?;

    Ok(HttpResponse::Ok().json(status))
}

/// Disable MFA endpoint (requires authentication)
#[utoipa::path(
    post,
    path = "/api/v1/auth/mfa/disable",
    tag = "Auth",
    request_body = DisableMfaRequest,
    responses(
        (status = 200, description = "MFA disabled successfully", body = MfaStatusResponse),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Invalid password"),
    ),
    security(("bearer_auth" = [])),
)]
pub async fn mfa_disable(
    auth_user: AuthUser,
    mfa_service: web::Data<MfaService>,
    user_service: web::Data<UserService>,
    body: web::Json<DisableMfaRequest>,
) -> Result<HttpResponse, ApiError> {
    let user_id: i64 = auth_user.0.user_id()?;
    let tenant_id = auth_user.0.tenant_id;

    // Verify current password before disabling MFA
    let user = user_service
        .get_user_by_username(&auth_user.0.username, tenant_id)
        .await?;

    crate::utils::password::verify_password(&body.password, &user.hashed_password)
        .map_err(|_| ApiError::Forbidden("Invalid password".to_string()))?;

    let status = mfa_service.disable_mfa(user_id, tenant_id).await?;

    Ok(HttpResponse::Ok().json(status))
}

/// Get MFA status endpoint (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/auth/mfa/status",
    tag = "Auth",
    responses(
        (status = 200, description = "MFA status", body = MfaStatusResponse),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
)]
pub async fn mfa_status(
    auth_user: AuthUser,
    mfa_service: web::Data<MfaService>,
) -> Result<HttpResponse, ApiError> {
    let user_id: i64 = auth_user.0.user_id()?;
    let tenant_id = auth_user.0.tenant_id;

    let status = mfa_service.get_mfa_status(user_id, tenant_id).await?;

    Ok(HttpResponse::Ok().json(status))
}

/// Verify MFA code during login (public, requires mfa_token)
#[utoipa::path(
    post,
    path = "/api/v1/auth/mfa/verify",
    tag = "Auth",
    request_body = VerifyMfaRequest,
    responses(
        (status = 200, description = "MFA verified, login successful", body = LoginResponse),
        (status = 401, description = "Invalid MFA token or code"),
    ),
)]
pub async fn mfa_verify(
    auth_service: web::Data<crate::domain::auth::AuthService>,
    mfa_service: web::Data<MfaService>,
    user_service: web::Data<UserService>,
    body: web::Json<VerifyMfaRequest>,
) -> Result<HttpResponse, ApiError> {
    // Decode the MFA token to get user identity
    let claims = mfa_service.decode_mfa_token(&body.mfa_token)?;
    let user_id = claims
        .sub
        .parse::<i64>()
        .map_err(|_| ApiError::InvalidToken("Invalid user ID in MFA token".to_string()))?;
    let tenant_id = claims.tenant_id;

    // Validate the MFA code
    let valid = mfa_service
        .validate_mfa_challenge(user_id, tenant_id, &body.code)
        .await?;

    if !valid {
        return Err(ApiError::Unauthorized("Invalid MFA code".to_string()));
    }

    // Get user and generate tokens
    let user = user_service
        .get_user_by_username(&claims.username, tenant_id)
        .await?;

    let tokens = auth_service
        .validate_token(&body.mfa_token)
        .ok()
        .and_then(|_| {
            auth_service
                .jwt_service
                .generate_tokens(user.id, tenant_id, user.username.clone(), user.role)
                .ok()
        });

    let tokens = match tokens {
        Some(t) => t,
        None => {
            // Generate fresh tokens if MFA token can't be used directly
            auth_service.jwt_service.generate_tokens(
                user.id,
                tenant_id,
                user.username.clone(),
                user.role,
            )?
        }
    };

    let user_response: crate::domain::user::model::UserResponse = user.into();

    Ok(HttpResponse::Ok().json(crate::domain::auth::LoginResponse {
        user: user_response,
        tokens,
    }))
}

/// Regenerate backup codes endpoint (requires authentication)
#[utoipa::path(
    post,
    path = "/api/v1/auth/mfa/regenerate-backup-codes",
    tag = "Auth",
    responses(
        (status = 200, description = "New backup codes generated", body = BackupCodesResponse),
        (status = 400, description = "MFA not enabled"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("bearer_auth" = [])),
)]
pub async fn mfa_regenerate_backup_codes(
    auth_user: AuthUser,
    mfa_service: web::Data<MfaService>,
) -> Result<HttpResponse, ApiError> {
    let user_id: i64 = auth_user.0.user_id()?;
    let tenant_id = auth_user.0.tenant_id;

    let response = mfa_service
        .regenerate_backup_codes(user_id, tenant_id)
        .await?;

    Ok(HttpResponse::Ok().json(response))
}

/// Configure MFA routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/auth/mfa")
            .route("/setup", web::post().to(mfa_setup))
            .route("/verify-setup", web::post().to(mfa_verify_setup))
            .route("/disable", web::post().to(mfa_disable))
            .route("/status", web::get().to(mfa_status))
            .route("/verify", web::post().to(mfa_verify))
            .route(
                "/regenerate-backup-codes",
                web::post().to(mfa_regenerate_backup_codes),
            ),
    );
}
