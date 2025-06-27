use {
    crate::{
        hardware_wallet::{HardwareWallet, HardwareWalletSettings, PubkeyDisplayMode},
        remote_wallet::{RemoteWalletError, RemoteWalletInfo},
    },
    solana_sdk::{derivation_path::DerivationPath, pubkey::Pubkey, signature::Signature},
    std::{fmt, rc::Rc},
};

/// Keystone hardware wallet implementation
pub struct KeystoneWallet {
    device_path: String,
    model: String,
    serial: String,
    firmware_version: String,
    settings: HardwareWalletSettings,
}

impl fmt::Debug for KeystoneWallet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HidDevice")
    }
}

impl KeystoneWallet {
    pub fn new(device_path: String) -> Result<Self, RemoteWalletError> {
        // TODO: Implement actual Keystone device connection
        // This is a placeholder implementation
        Ok(Self {
            device_path,
            model: "Keystone Pro".to_string(),
            serial: "unknown".to_string(),
            firmware_version: "1.0.0".to_string(),
            settings: HardwareWalletSettings {
                enable_blind_signing: false,
                pubkey_display_mode: PubkeyDisplayMode::Short,
                auto_lock_delay: Some(300), // 5 minutes
            },
        })
    }

    /// Keystone specific APDU commands
    const GET_PUBKEY: u8 = 0x02;
    const SIGN_MESSAGE: u8 = 0x04;
    const GET_DEVICE_INFO: u8 = 0x06;
}

impl HardwareWallet for KeystoneWallet {
    fn name(&self) -> &str {
        "Keystone hardware wallet"
    }

    fn manufacturer(&self) -> &str {
        "keystone"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn serial(&self) -> &str {
        &self.serial
    }

    fn device_path(&self) -> &str {
        &self.device_path
    }

    fn firmware_version(&self) -> Result<String, RemoteWalletError> {
        Ok(self.firmware_version.clone())
    }

    fn get_pubkey(
        &self,
        derivation_path: &DerivationPath,
        confirm_key: bool,
    ) -> Result<Pubkey, RemoteWalletError> {
        // TODO: Implement actual Keystone APDU communication
        // This would involve:
        // 1. Serializing the derivation path
        // 2. Sending APDU command to device
        // 3. Receiving and parsing the response
        // 4. Converting to Pubkey

        // Placeholder implementation
        Err(RemoteWalletError::Protocol(
            "Keystone get_pubkey not implemented",
        ))
    }

    fn sign_message(
        &self,
        derivation_path: &DerivationPath,
        data: &[u8],
    ) -> Result<Signature, RemoteWalletError> {
        // TODO: Implement actual Keystone message signing
        // This would involve:
        // 1. Serializing the derivation path and message
        // 2. Sending APDU command to device
        // 3. Receiving and parsing the signature
        // 4. Converting to Signature

        // Placeholder implementation
        Err(RemoteWalletError::Protocol(
            "Keystone sign_message not implemented",
        ))
    }

    fn sign_offchain_message(
        &self,
        derivation_path: &DerivationPath,
        message: &[u8],
    ) -> Result<Signature, RemoteWalletError> {
        // For Keystone, off-chain messages might be handled differently
        // or might not be supported
        Err(RemoteWalletError::Protocol(
            "Keystone off-chain signing not supported",
        ))
    }

    fn get_device_info(&self) -> Result<RemoteWalletInfo, RemoteWalletError> {
        Ok(RemoteWalletInfo {
            model: self.model.clone(),
            manufacturer: crate::locator::Manufacturer::Keystone,
            serial: self.serial.clone(),
            host_device_path: self.device_path.clone(),
            pubkey: Pubkey::default(), // Would be populated after first get_pubkey call
            error: None,
        })
    }

    fn is_connected(&self) -> bool {
        // TODO: Implement actual connection check
        true
    }
}

/// Check if the detected device is a valid Keystone device
pub fn is_valid_keystone(vendor_id: u16, product_id: u16) -> bool {
    const KEYSTONE_VID: u16 = 0x1209;
    const KEYSTONE_PIDS: u16 = 0x3001;

    vendor_id == KEYSTONE_VID && product_id == KEYSTONE_PIDS
}
