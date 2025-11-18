pub mod department_repository;
pub mod major_repository;
pub mod mfa_verify_result;
pub mod otp_verify_repository;
pub mod request_repository;
pub mod score_repository;
pub mod user_mfa_repository;
pub mod user_repository;
pub mod wallet_repository;
pub mod file_upload_repository;

pub use department_repository::{DepartmentRepository, DepartmentUpdate};
pub use major_repository::{MajorRepository, MajorUpdate};
pub use otp_verify_repository::OtpVerifyRepository;
pub use request_repository::RequestRepository;
pub use score_repository::ScoreRepository;
pub use user_mfa_repository::UserMfaRepository;
pub use user_repository::UserRepository;
pub use wallet_repository::WalletRepository;
