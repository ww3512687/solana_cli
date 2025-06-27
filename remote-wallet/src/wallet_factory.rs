use {
    crate::{
        hardware_wallet::HardwareWallet,
        keystone::KeystoneWallet,
        ledger::LedgerWallet,
        remote_wallet::{RemoteWalletError, RemoteWalletInfo, RemoteWalletType},
        // trezor::TrezorWallet,
    },
    std::rc::Rc,
};

/// Factory for creating hardware wallet instances
pub struct WalletFactory;

impl WalletFactory {
    /// Create a hardware wallet based on the device info
    pub fn create_wallet(
        device_info: &RemoteWalletInfo,
        device_path: String,
    ) -> Result<RemoteWalletType, RemoteWalletError> {
        match device_info.manufacturer {
            crate::locator::Manufacturer::Ledger => {
                // For Ledger, we need the actual HID device
                // This should be handled differently since Ledger requires HID connection
                Err(RemoteWalletError::Protocol(
                    "Ledger wallet creation requires HID device",
                ))
            }
            crate::locator::Manufacturer::Keystone => {
                let keystone = KeystoneWallet::new(device_path)?;
                Ok(RemoteWalletType::Keystone(Rc::new(keystone)))
            }
            // crate::locator::Manufacturer::Trezor => {
            //     let trezor = TrezorWallet::new(device_path)?;
            //     Ok(RemoteWalletType::Trezor(Rc::new(trezor)))
            // }
            crate::locator::Manufacturer::Unknown => Err(RemoteWalletError::DeviceTypeMismatch),
        }
    }

    /// Get the supported manufacturers
    pub fn supported_manufacturers() -> Vec<&'static str> {
        vec!["ledger", "keystone", "trezor"]
    }

    /// Check if a manufacturer is supported
    pub fn is_supported(manufacturer: &str) -> bool {
        Self::supported_manufacturers().contains(&manufacturer)
    }
}

/// Helper functions for wallet operations
pub mod wallet_helpers {
    use super::*;

    /// Get wallet name by type
    pub fn get_wallet_name(wallet_type: &RemoteWalletType) -> &str {
        match wallet_type {
            RemoteWalletType::Ledger(_) => "Ledger",
            RemoteWalletType::Keystone(_) => "Keystone",
            // RemoteWalletType::Trezor(_) => "Trezor",
        }
    }

    /// Get wallet manufacturer by type
    pub fn get_wallet_manufacturer(wallet_type: &RemoteWalletType) -> &str {
        match wallet_type {
            RemoteWalletType::Ledger(_) => "ledger",
            RemoteWalletType::Keystone(_) => "keystone",
            // RemoteWalletType::Trezor(_) => "trezor",
        }
    }
}
