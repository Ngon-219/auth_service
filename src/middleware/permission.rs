use axum::http::StatusCode;
use do_an_lib::structs::token_claims::{TokenClaims, UserRole};

/// Check if user has required role(s)
pub fn has_role(
    claims: &TokenClaims,
    required_roles: &[UserRole],
) -> Result<(), (StatusCode, String)> {
    if required_roles.contains(&claims.role) {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            format!("Access denied. Required role: {:?}", required_roles),
        ))
    }
}

/// Check if user is admin
pub fn is_admin(claims: &TokenClaims) -> Result<(), (StatusCode, String)> {
    has_role(claims, &[UserRole::ADMIN])
}

/// Check if user is admin or manager
pub fn is_admin_or_manager(claims: &TokenClaims) -> Result<(), (StatusCode, String)> {
    has_role(claims, &[UserRole::ADMIN, UserRole::MANAGER])
}

/// Check if user is admin, manager, or teacher
pub fn is_staff(claims: &TokenClaims) -> Result<(), (StatusCode, String)> {
    has_role(
        claims,
        &[UserRole::ADMIN, UserRole::MANAGER, UserRole::TEACHER],
    )
}

/// Check if user can access resource (is owner or has admin/manager role)
pub fn can_access_user_resource(
    claims: &TokenClaims,
    target_user_id: &str,
) -> Result<(), (StatusCode, String)> {
    // Admin can access everything
    if claims.role == UserRole::ADMIN {
        return Ok(());
    }

    // User can access their own resources
    if claims.user_id == target_user_id {
        return Ok(());
    }

    Err((
        StatusCode::FORBIDDEN,
        "You don't have permission to access this resource".to_string(),
    ))
}

/// Check if user can modify another user (based on use case spec)
/// UC10: Admin can modify any account
/// UC17: Admin can modify student accounts
/// UC18: Manager can delete student accounts
pub fn can_modify_user(
    claims: &TokenClaims,
    target_role: &UserRole,
) -> Result<(), (StatusCode, String)> {
    match claims.role {
        // Admin can modify anyone
        UserRole::ADMIN => Ok(()),
        // Manager can only modify students
        UserRole::MANAGER => {
            if *target_role == UserRole::STUDENT {
                Ok(())
            } else {
                Err((
                    StatusCode::FORBIDDEN,
                    "Managers can only modify student accounts".to_string(),
                ))
            }
        }
        // Others cannot modify
        _ => Err((
            StatusCode::FORBIDDEN,
            "You don't have permission to modify user accounts".to_string(),
        )),
    }
}
