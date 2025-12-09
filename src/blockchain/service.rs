use crate::blockchain::contract::DataStorage;
use crate::config::APP_CONFIG;
use anyhow::{Context, Result};
use ethers::prelude::*;
use ethers::providers::{Http, Provider};
use ethers::signers::{LocalWallet, Signer};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct BlockchainService {
    contract: DataStorage<SignerMiddleware<Provider<Http>, LocalWallet>>,
}

impl BlockchainService {
    pub async fn new() -> Result<Self> {
        let provider = Provider::<Http>::try_from(&APP_CONFIG.blockchain_rpc_url)
            .context("Failed to create provider")?;

        let admin_private_key = APP_CONFIG.admin_private_key.clone();
        let wallet: LocalWallet = admin_private_key.parse().context("Failed to parse private key")?;

        let chain_id = provider.get_chainid().await?;
        let wallet = wallet.with_chain_id(chain_id.as_u64());

        let client = SignerMiddleware::new(provider, wallet);
        let client = Arc::new(client);

        let contract_address: Address = APP_CONFIG
            .data_storage_contract_address
            .parse()
            .context("Failed to parse contract address")?;

        let contract = DataStorage::new(contract_address, client);

        Ok(Self { contract })
    }

    /// Generate a new Ethereum wallet
    pub fn generate_wallet() -> Result<(String, String)> {
        // Generate a random wallet
        let wallet = LocalWallet::new(&mut rand::thread_rng());

        let address = format!("{:?}", wallet.address());
        // Get private key as hex string
        let private_key = wallet.signer().to_bytes();
        let private_key_hex = hex::encode(private_key);

        Ok((address, format!("0x{}", private_key_hex)))
    }

    /// Register a single student on the blockchain
    pub async fn register_student(
        &self,
        wallet_address: &str,
        student_code: &str,
        full_name: &str,
        email: &str,
    ) -> Result<U256> {
        let address: Address = wallet_address
            .parse()
            .context("Failed to parse wallet address")?;

        let tx = self
            .contract
            .register_student(
                address,
                student_code.to_string(),
                full_name.to_string(),
                email.to_string(),
            )
            .send()
            .await
            .context("Failed to send register student transaction")?
            .await
            .context("Failed to wait for transaction confirmation")?
            .ok_or_else(|| anyhow::anyhow!("Transaction receipt not found"))?;

        // Parse the event to get the student ID
        let logs = tx.logs;
        for log in logs {
            if let Ok(event) = self.contract.decode_event::<(U256, Address, String)>(
                "StudentRegistered",
                log.topics.clone(),
                log.data.clone(),
            ) {
                return Ok(event.0);
            }
        }

        Err(anyhow::anyhow!(
            "StudentRegistered event not found in transaction"
        ))
    }

    /// Register multiple students on the blockchain in batch
    pub async fn register_students_batch(
        &self,
        wallet_addresses: Vec<String>,
        student_codes: Vec<String>,
        full_names: Vec<String>,
        emails: Vec<String>,
    ) -> Result<()> {
        let addresses: Result<Vec<Address>> = wallet_addresses
            .iter()
            .map(|addr| addr.parse().context("Failed to parse wallet address"))
            .collect();

        let addresses = addresses?;

        let tx = self
            .contract
            .register_students_batch(addresses, student_codes, full_names, emails)
            .send()
            .await
            .context("Failed to send batch register transaction")?
            .await
            .context("Failed to wait for transaction confirmation")?;

        if tx.is_none() {
            return Err(anyhow::anyhow!("Transaction receipt not found"));
        }

        Ok(())
    }

    /// Assign role to a user on the blockchain
    pub async fn assign_role(&self, user_address: &str, role: u8) -> Result<()> {
        let address: Address = user_address
            .parse()
            .context("Failed to parse user address")?;

        let _tx = self
            .contract
            .assign_role(address, role)
            .send()
            .await
            .context("Failed to send assign role transaction")?
            .await
            .context("Failed to wait for transaction confirmation")?;

        Ok(())
    }

    /// Add a manager to the system
    pub async fn add_manager(&self, manager_address: &str) -> Result<()> {
        let address: Address = manager_address
            .parse()
            .context("Failed to parse manager address")?;

        let _tx = self
            .contract
            .add_manager(address)
            .send()
            .await
            .context("Failed to send add manager transaction")?
            .await
            .context("Failed to wait for transaction confirmation")?;

        Ok(())
    }

    /// Remove a manager from the system
    pub async fn remove_manager(&self, manager_address: &str) -> Result<()> {
        let address: Address = manager_address
            .parse()
            .context("Failed to parse manager address")?;

        let _tx = self
            .contract
            .remove_manager(address)
            .send()
            .await
            .context("Failed to send remove manager transaction")?
            .await
            .context("Failed to wait for transaction confirmation")?;

        Ok(())
    }

    /// Get all managers from the system
    pub async fn get_all_managers(&self) -> Result<Vec<String>> {
        let managers = self
            .contract
            .get_all_managers()
            .call()
            .await
            .context("Failed to get all managers")?;

        Ok(managers.iter().map(|addr| format!("{:?}", addr)).collect())
    }

    /// Get manager count
    pub async fn get_manager_count(&self) -> Result<u64> {
        let count = self
            .contract
            .get_manager_count()
            .call()
            .await
            .context("Failed to get manager count")?;

        Ok(count.as_u64())
    }

    /// Check if an address is a manager
    pub async fn is_manager(&self, address: &str) -> Result<bool> {
        let addr: Address = address.parse().context("Failed to parse address")?;

        let is_mgr = self
            .contract
            .is_manager(addr)
            .call()
            .await
            .context("Failed to check if manager")?;

        Ok(is_mgr)
    }

    /// Deactivate a student
    pub async fn deactivate_student(&self, student_id: u64) -> Result<()> {
        let _tx = self
            .contract
            .deactivate_student(U256::from(student_id))
            .send()
            .await
            .context("Failed to send deactivate student transaction")?
            .await
            .context("Failed to wait for transaction confirmation")?;

        Ok(())
    }

    /// Activate a student
    pub async fn activate_student(&self, student_id: u64) -> Result<()> {
        let _tx = self
            .contract
            .activate_student(U256::from(student_id))
            .send()
            .await
            .context("Failed to send activate student transaction")?
            .await
            .context("Failed to wait for transaction confirmation")?;

        Ok(())
    }

    /// Get student ID by wallet address
    pub async fn get_student_id_by_address(&self, address: &str) -> Result<u64> {
        let addr: Address = address.parse().context("Failed to parse address")?;

        let student_id = self
            .contract
            .get_student_id_by_address(addr)
            .call()
            .await
            .context("Failed to get student ID by address")?;

        Ok(student_id.as_u64())
    }

    /// Get student ID by student code
    pub async fn get_student_id_by_code(&self, student_code: &str) -> Result<u64> {
        let student_id = self
            .contract
            .get_student_id_by_code(student_code.to_string())
            .call()
            .await
            .context("Failed to get student ID by code")?;

        Ok(student_id.as_u64())
    }

    /// Check if a student is active
    pub async fn is_active_student(&self, address: &str) -> Result<bool> {
        let addr: Address = address.parse().context("Failed to parse address")?;

        let is_active = self
            .contract
            .is_active_student(addr)
            .call()
            .await
            .context("Failed to check if student is active")?;

        Ok(is_active)
    }

    /// Get user role
    pub async fn get_user_role(&self, address: &str) -> Result<u8> {
        let addr: Address = address.parse().context("Failed to parse address")?;

        let role = self
            .contract
            .get_user_role(addr)
            .call()
            .await
            .context("Failed to get user role")?;

        Ok(role)
    }

    /// Check if user has specific role
    pub async fn has_role(&self, address: &str, role: u8) -> Result<bool> {
        let addr: Address = address.parse().context("Failed to parse address")?;

        let has_role = self
            .contract
            .has_role(addr, role)
            .call()
            .await
            .context("Failed to check if user has role")?;

        Ok(has_role)
    }

    /// Check if user is teacher or admin
    pub async fn is_teacher_or_admin(&self, address: &str) -> Result<bool> {
        let addr: Address = address.parse().context("Failed to parse address")?;

        let is_teacher_or_admin = self
            .contract
            .is_teacher_or_admin(addr)
            .call()
            .await
            .context("Failed to check if teacher or admin")?;

        Ok(is_teacher_or_admin)
    }

    /// Get total number of students
    pub async fn get_total_students(&self) -> Result<u64> {
        let total = self
            .contract
            .get_total_students()
            .call()
            .await
            .context("Failed to get total students")?;

        Ok(total.as_u64())
    }

    /// Get contract information
    pub async fn get_contract_info(&self) -> Result<(String, u64, u64)> {
        let (owner, student_count, manager_count) = self
            .contract
            .get_contract_info()
            .call()
            .await
            .context("Failed to get contract info")?;

        Ok((
            format!("{:?}", owner),
            student_count.as_u64(),
            manager_count.as_u64(),
        ))
    }

    /// Get student details by ID
    pub async fn get_student(&self, student_id: u64) -> Result<StudentInfo> {
        let student = self
            .contract
            .get_student(U256::from(student_id))
            .call()
            .await
            .context("Failed to get student")?;

        Ok(StudentInfo {
            id: student.0.as_u64(),
            wallet_address: format!("{:?}", student.1),
            student_code: student.2,
            full_name: student.3,
            email: student.4,
            is_active: student.5,
            registered_at: student.6.as_u64(),
        })
    }
}

/// Student information struct
#[derive(Debug, Clone)]
pub struct StudentInfo {
    pub id: u64,
    pub wallet_address: String,
    pub student_code: String,
    pub full_name: String,
    pub email: String,
    pub is_active: bool,
    pub registered_at: u64,
}
