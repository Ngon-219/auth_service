use utoipa::Modify;
use utoipa::OpenApi;
use utoipa::openapi::security::HttpAuthScheme;
use utoipa::openapi::security::HttpBuilder;
use utoipa::openapi::security::SecurityScheme;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::routes::health::route::health_check,
        crate::routes::auth::route::login,
        crate::routes::profile::route::get_profile,
        crate::routes::users::route::create_user,
        crate::routes::users::route::create_users_bulk,
        crate::routes::users::route::get_all_users,
        crate::routes::users::route::get_user_by_id,
        crate::routes::users::route::update_user,
        crate::routes::users::route::delete_user,
        crate::routes::departments::route::create_department,
        crate::routes::departments::route::get_all_departments,
        crate::routes::departments::route::get_department,
        crate::routes::departments::route::update_department,
        crate::routes::departments::route::delete_department,
        crate::routes::majors::route::create_major,
        crate::routes::majors::route::get_all_majors,
        crate::routes::majors::route::get_major,
        crate::routes::majors::route::update_major,
        crate::routes::majors::route::delete_major,
        crate::routes::managers::route::add_manager,
        crate::routes::managers::route::remove_manager,
        crate::routes::managers::route::get_all_managers,
        crate::routes::managers::route::check_manager,
        crate::routes::students::route::get_student_by_id,
        crate::routes::students::route::get_student_id_by_address,
        crate::routes::students::route::get_student_id_by_code,
        crate::routes::students::route::deactivate_student,
        crate::routes::students::route::activate_student,
        crate::routes::students::route::check_student_active,
        crate::routes::students::route::get_system_info,
        crate::routes::user_mfa::route::req_enable_mfa,
        crate::routes::user_mfa::route::enable_mfa,
        crate::routes::user_mfa::route::verify_mfa_code_test,
    ),
    components(
        schemas(
            crate::routes::auth::dto::LoginRequest,
            crate::routes::auth::dto::LoginResponse,
            crate::routes::profile::dto::ProfileResponse,
            crate::routes::users::dto::CreateUserRequest,
            crate::routes::users::dto::UpdateUserRequest,
            crate::routes::users::dto::UserResponse,
            crate::routes::users::dto::UserDetailResponse,
            crate::routes::users::dto::UserListResponse,
            crate::routes::users::dto::BulkUserResponse,
            crate::routes::users::dto::BulkUserError,
            crate::routes::departments::dto::CreateDepartmentRequest,
            crate::routes::departments::dto::UpdateDepartmentRequest,
            crate::routes::departments::dto::DepartmentResponse,
            crate::routes::departments::dto::DepartmentListResponse,
            crate::routes::majors::dto::CreateMajorRequest,
            crate::routes::majors::dto::UpdateMajorRequest,
            crate::routes::majors::dto::MajorResponse,
            crate::routes::majors::dto::MajorListResponse,
            crate::routes::managers::dto::AddManagerRequest,
            crate::routes::managers::dto::RemoveManagerRequest,
            crate::routes::managers::dto::ManagerResponse,
            crate::routes::managers::dto::ManagerListResponse,
            crate::routes::managers::dto::CheckManagerRequest,
            crate::routes::students::dto::StudentIdRequest,
            crate::routes::students::dto::StudentAddressRequest,
            crate::routes::students::dto::StudentCodeRequest,
            crate::routes::students::dto::StudentInfoResponse,
            crate::routes::students::dto::StudentStatusResponse,
            crate::routes::students::dto::StudentIdResponse,
            crate::routes::students::dto::SystemInfoResponse,
            crate::routes::user_mfa::dto::EnableMfaRequestDto,
            crate::routes::user_mfa::dto::EnableMfaResponseDto,
            crate::routes::user_mfa::dto::ReqEnableMfaResponseDto,
            crate::routes::user_mfa::dto::VerifyMfaCodeTestRequestDto,
            crate::routes::user_mfa::dto::VerifyMfaCodeTestResponseDto,
            crate::entities::sea_orm_active_enums::RoleEnum,
        ),
    ),
    modifiers(&SecurityModifier),
    tags(
        (name = "Authentication", description = "Login and JWT token endpoints"),
        (name = "Profile", description = "Current user profile with blockchain info"),
        (name = "Users", description = "User management endpoints"),
        (name = "Departments", description = "Department CRUD endpoints"),
        (name = "Majors", description = "Major CRUD endpoints"),
        (name = "Managers", description = "Manager management endpoints"),
        (name = "Students", description = "Student information endpoints"),
        (name = "System", description = "System information endpoints"),
        (name = "security-settings", description = "Security settings and MFA endpoints"),
        (name = "health", description = "Health check endpoints")
    ),
)]
pub struct ApiDoc;

struct SecurityModifier;
impl Modify for SecurityModifier {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap();
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
        components.add_security_scheme(
            "basic_auth",
            SecurityScheme::Http(HttpBuilder::new().scheme(HttpAuthScheme::Basic).build()),
        );
    }
}
