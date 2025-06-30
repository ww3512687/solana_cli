use crate::errors::RemoteWalletError;
use crate::locator::Manufacturer;
use solana_sdk::{
    derivation_path::DerivationPath,
    pubkey::Pubkey,
    signature::Signature,
};

/// 硬件钱包信息
#[derive(Debug, Clone)]
pub struct RemoteWalletInfo {
    pub model: String,
    pub manufacturer: Manufacturer,
    pub serial: String,
    pub host_device_path: String,
    pub pubkey: Pubkey,
    pub error: Option<RemoteWalletError>,
}

impl RemoteWalletInfo {
    pub fn parse_locator(locator: crate::locator::Locator) -> Self {
        Self {
            manufacturer: locator.manufacturer,
            pubkey: locator.pubkey.unwrap_or_default(),
            model: "unknown".to_string(),
            serial: "".to_string(),
            host_device_path: "".to_string(),
            error: None,
        }
    }

    pub fn get_pretty_path(&self) -> String {
        format!("usb://{:?}", self.manufacturer)
    }

    pub fn matches(&self, other: &Self) -> bool {
        self.manufacturer == other.manufacturer
            && (self.pubkey == other.pubkey
                || self.pubkey == Pubkey::default()
                || other.pubkey == Pubkey::default())
    }
}

/// 硬件钱包通用接口
pub trait HardwareWallet {
    fn name(&self) -> &str;
    
    /// 读取设备信息
    fn read_device(&mut self) -> Result<RemoteWalletInfo, RemoteWalletError>;
    
    /// 获取公钥
    fn get_pubkey(&self, derivation_path: &DerivationPath, confirm_key: bool) -> Result<Pubkey, RemoteWalletError>;
    
    /// 签名消息
    fn sign_message(&self, derivation_path: &DerivationPath, data: &[u8]) -> Result<Signature, RemoteWalletError>;
    
    /// 签名离线消息
    fn sign_offchain_message(&self, derivation_path: &DerivationPath, message: &[u8]) -> Result<Signature, RemoteWalletError>;
} 