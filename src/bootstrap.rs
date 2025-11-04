use anyhow::{Context, Result};
use chrono::Utc;
use ethers::signers::{LocalWallet, Signer};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use crate::config::APP_CONFIG;
use crate::entities::{sea_orm_active_enums::RoleEnum, user, wallet};
use crate::utils::encryption::encrypt_private_key;

pub async fn initialize_admin_user(db: &DatabaseConnection) -> Result<()> {
    let admin_email: &str = &APP_CONFIG.admin_email;
    let default_password: &str = &APP_CONFIG.admin_password;

    let existing_admin = user::Entity::find()
        .filter(user::Column::Email.eq(admin_email))
        .one(db)
        .await
        .context("Failed to check existing admin")?;

    if existing_admin.is_some() {
        tracing::info!("Admin user already exists, skipping initialization");
        return Ok(());
    }

    tracing::info!("Creating default admin user...");

    let admin_wallet: LocalWallet = APP_CONFIG
        .admin_private_key
        .parse()
        .context("Failed to parse admin private key")?;

    let wallet_address = format!("{:?}", admin_wallet.address());
    let private_key = APP_CONFIG.admin_private_key.clone();

    let encrypted_private_key = encrypt_private_key(&private_key, &APP_CONFIG.encryption_key)
        .context("Failed to encrypt admin private key")?;

    let hashed_password = bcrypt::hash(default_password, bcrypt::DEFAULT_COST)
        .context("Failed to hash admin password")?;

    let user_id = Uuid::new_v4();
    let wallet_id = Uuid::new_v4();
    let now = Utc::now().naive_utc();

    let admin_user = user::ActiveModel {
        user_id: Set(user_id),
        first_name: Set("System".to_string()),
        last_name: Set("Administrator".to_string()),
        address: Set("System".to_string()),
        email: Set(admin_email.to_string()),
        password: Set(hashed_password),
        is_priority: Set(true),
        cccd: Set("ADMIN000000".to_string()),
        phone_number: Set("0000000000".to_string()),
        is_first_login: Set(true),
        create_at: Set(now),
        update_at: Set(now),
        role: Set(RoleEnum::Admin),
    };

    admin_user
        .insert(db)
        .await
        .context("Failed to insert admin user")?;

    // Create admin wallet
    let admin_wallet = wallet::ActiveModel {
        wallet_id: Set(wallet_id),
        user_id: Set(user_id),
        address: Set(wallet_address.clone()),
        private_key: Set(encrypted_private_key),
        chain_type: Set("ethereum".to_string()),
        public_key: Set(wallet_address.clone()),
        status: Set("active".to_string()),
        network_id: Set("11155111".to_string()), // Sepolia testnet
        last_used_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };

    admin_wallet
        .insert(db)
        .await
        .context("Failed to insert admin wallet")?;

    tracing::info!("✅ Admin user created successfully!");
    tracing::info!("  Email: {}", admin_email);
    tracing::info!("  Password: {}", default_password);
    tracing::info!("  Wallet: {}", wallet_address);
    tracing::warn!("⚠️  Please change the default password after first login!");

    Ok(())
}
