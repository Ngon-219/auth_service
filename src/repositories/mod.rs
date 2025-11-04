pub mod user_repository;
pub mod user_mfa_repository;
pub mod otp_verify_repository;
pub mod wallet_repository;
pub mod department_repository;
pub mod major_repository;

pub use user_repository::UserRepository;
pub use user_mfa_repository::UserMfaRepository;
pub use otp_verify_repository::OtpVerifyRepository;
pub use wallet_repository::WalletRepository;
pub use department_repository::{DepartmentRepository, DepartmentUpdate};
pub use major_repository::{MajorRepository, MajorUpdate};
