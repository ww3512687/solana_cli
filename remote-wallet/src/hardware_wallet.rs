use {
    crate::remote_wallet::{RemoteWalletError, RemoteWalletInfo},
    solana_sdk::{derivation_path::DerivationPath, pubkey::Pubkey, signature::Signature},
};

/// Common trait for all hardware wallets
pub trait HardwareWallet {
    /// Get the name of the hardware wallet
    fn name(&self) -> &str;

    /// Get the manufacturer of the hardware wallet
    fn manufacturer(&self) -> &str;

    /// Get the model of the hardware wallet
    fn model(&self) -> &str;

    /// Get the serial number of the hardware wallet
    fn serial(&self) -> &str;

    /// Get the device path
    fn device_path(&self) -> &str;

    /// Get the firmware version
    fn firmware_version(&self) -> Result<String, RemoteWalletError>;

    /// Get the public key for a given derivation path
    fn get_pubkey(
        &self,
        derivation_path: &DerivationPath,
        confirm_key: bool,
    ) -> Result<Pubkey, RemoteWalletError>;

    /// Sign a message with the given derivation path
    fn sign_message(
        &self,
        derivation_path: &DerivationPath,
        data: &[u8],
    ) -> Result<Signature, RemoteWalletError>;

    /// Sign an off-chain message with the given derivation path
    fn sign_offchain_message(
        &self,
        derivation_path: &DerivationPath,
        message: &[u8],
    ) -> Result<Signature, RemoteWalletError>;

    /// Get device information
    fn get_device_info(&self) -> Result<RemoteWalletInfo, RemoteWalletError>;

    /// Check if the device is connected and responsive
    fn is_connected(&self) -> bool;
}

/// Common settings that can be configured on hardware wallets
#[derive(Debug, Clone)]
pub struct HardwareWalletSettings {
    pub enable_blind_signing: bool,
    pub pubkey_display_mode: PubkeyDisplayMode,
    pub auto_lock_delay: Option<u32>, // in seconds
}

#[derive(Debug, Clone)]
pub enum PubkeyDisplayMode {
    Short,
    Long,
    Full,
}

/// Common error types for hardware wallet operations
#[derive(Debug, thiserror::Error)]
pub enum HardwareWalletError {
    #[error("device not connected")]
    DeviceNotConnected,

    #[error("device locked")]
    DeviceLocked,

    #[error("user cancelled operation")]
    UserCancelled,

    #[error("invalid derivation path")]
    InvalidDerivationPath,

    #[error("message too long")]
    MessageTooLong,

    #[error("unsupported operation")]
    UnsupportedOperation,

    #[error("protocol error: {0}")]
    Protocol(String),

    #[error("communication error: {0}")]
    Communication(String),
}
