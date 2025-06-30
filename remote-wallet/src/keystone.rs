use {
    crate::{
        hardware_wallet::{HardwareWallet, HardwareWalletSettings, PubkeyDisplayMode},
        keystone_protocol::{
            CommandType, EAPDURequestPayload, EAPDUResponsePayload, StatusEnum,
            KEYSTONE_PROTOCOL_HEADER,
        },
        remote_wallet::{RemoteWalletError, RemoteWalletInfo},
        ur_protocol::{UR, URProtocol, URProtocolError},
    },
    log::*,
    serde::{Deserialize, Serialize},
    solana_sdk::{derivation_path::DerivationPath, pubkey::Pubkey, signature::Signature},
    std::{fmt, rc::Rc, time::Duration},
    thiserror::Error,
    std::str::FromStr,
};

#[cfg(feature = "hidapi")]
use hidapi::HidDevice;

#[derive(Error, Debug)]
pub enum KeystoneError {
    #[error("USB communication error: {0}")]
    UsbError(String),
    #[error("Protocol error: {0}")]
    ProtocolError(String),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("Device not connected")]
    NotConnected,
    #[error("Operation rejected by user")]
    UserRejected,
    #[error("Device is locked")]
    DeviceLocked,
    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),
    #[error("UR protocol error: {0}")]
    URProtocolError(#[from] URProtocolError),
}

impl From<KeystoneError> for RemoteWalletError {
    fn from(err: KeystoneError) -> Self {
        match err {
            KeystoneError::UsbError(msg) => RemoteWalletError::Hid(msg),
            KeystoneError::ProtocolError(msg) => RemoteWalletError::InvalidInput(msg),
            KeystoneError::InvalidResponse(msg) => RemoteWalletError::InvalidInput(msg),
            KeystoneError::NotConnected => RemoteWalletError::NoDeviceFound,
            KeystoneError::UserRejected => RemoteWalletError::UserCancel,
            KeystoneError::DeviceLocked => RemoteWalletError::InvalidInput("Device is locked".to_string()),
            KeystoneError::UnsupportedOperation(msg) => RemoteWalletError::InvalidInput(msg),
            KeystoneError::URProtocolError(e) => RemoteWalletError::InvalidInput(e.to_string()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct DeviceInfoResponse {
    firmwareVersion: String,
    walletMFP: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExportAddressResponse {
    payload: String,
}

/// Keystone hardware wallet implementation
pub struct KeystoneWallet {
    #[cfg(feature = "hidapi")]
    device: Option<HidDevice>,
    device_path: String,
    model: String,
    serial: String,
    firmware_version: String,
    settings: HardwareWalletSettings,
}

impl fmt::Debug for KeystoneWallet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "KeystoneWallet")
    }
}

impl KeystoneWallet {
    pub fn new(device_path: String) -> Result<Self, RemoteWalletError> {
        #[cfg(feature = "hidapi")]
        {
            let device = Self::connect_to_device(&device_path)?;
            println!("device: {:?}", device);
            let mut wallet = Self {
                device: Some(device),
                device_path,
                model: "Keystone Pro".to_string(),
                serial: "unknown".to_string(),
                firmware_version: "1.0.0".to_string(),
                settings: HardwareWalletSettings {
                    enable_blind_signing: false,
                    pubkey_display_mode: PubkeyDisplayMode::Short,
                    auto_lock_delay: Some(300), // 5 minutes
                },
            };

            println!("wallet: {:?}", wallet);
            // Get device info to populate model and firmware version
            let device_info = wallet.get_device_info_internal()?;
            println!("device_info: {:?}", device_info);
            wallet.model = device_info.model.clone();
            // wallet.firmware_version = device_info.firmware_version.clone();

            Ok(wallet)
        }

        #[cfg(not(feature = "hidapi"))]
        {
            Err(RemoteWalletError::Hid(
                "hidapi crate compilation disabled in solana-remote-wallet.".to_string(),
            ))
        }
    }

    #[cfg(feature = "hidapi")]
    fn connect_to_device(device_path: &str) -> Result<HidDevice, KeystoneError> {
        let hidapi = hidapi::HidApi::new()
            .map_err(|e| KeystoneError::UsbError(format!("Failed to initialize HID API: {}", e)))?;

        // Parse device path to get vendor and product IDs
        // For now, use the known Keystone VID/PID
        const KEYSTONE_VID: u16 = 0x1209;
        const KEYSTONE_PID: u16 = 0x3001;

        let device = hidapi.open(KEYSTONE_VID, KEYSTONE_PID)
            .map_err(|e| KeystoneError::UsbError(format!("Failed to open Keystone device: {}", e)))?;

        Ok(device)
    }

    #[cfg(feature = "hidapi")]
    fn send_apdu_command(
        &self,
        command: CommandType,
        data: Option<&[u8]>,
    ) -> Result<EAPDUResponsePayload, KeystoneError> {
        let device = self.device.as_ref()
            .ok_or(KeystoneError::NotConnected)?;

        let request_id = 0u16;
        let data = data.unwrap_or(&[]);

        let request = EAPDURequestPayload {
            cla: KEYSTONE_PROTOCOL_HEADER,
            data: data.to_vec(),
            data_len: data.len() as u32,
            request_id,
            command_type: command,
        };

        // Serialize and send the request
        let serialized = self.serialize_request(&request)?;
        self.send_data(device, &serialized)?;

        // Receive and parse the response
        let response_data = self.receive_data(device)?;
        let response = self.deserialize_response(&response_data)?;

        // Check for errors
        match response.status {
            StatusEnum::RspSuccessCode => Ok(response),
            StatusEnum::PrsExportAddressRejected => Err(KeystoneError::UserRejected),
            StatusEnum::PrsExportAddressDisallowed => Err(KeystoneError::DeviceLocked),
            StatusEnum::PrsExportAddressUnsupportedChain => {
                Err(KeystoneError::UnsupportedOperation("Unsupported chain".to_string()))
            }
            _ => Err(KeystoneError::ProtocolError(format!(
                "Command failed with status: {:?}",
                response.status
            ))),
        }
    }

    #[cfg(feature = "hidapi")]
    fn send_data(&self, device: &HidDevice, data: &[u8]) -> Result<(), KeystoneError> {
        // Keystone uses a packet-based protocol similar to Ledger
        const HID_PACKET_SIZE: usize = 64;
        const HID_PREFIX_ZERO: usize = 0; // No prefix on non-Windows

        let mut offset = 0;
        let mut sequence_number = 0;

        while offset < data.len() {
            let mut packet = [0u8; HID_PACKET_SIZE];
            let header_size = if sequence_number == 0 { 5 } else { 5 };
            let available_space = HID_PACKET_SIZE - header_size - HID_PREFIX_ZERO;
            let chunk_size = std::cmp::min(available_space, data.len() - offset);

            // Build packet header
            let header_start = HID_PREFIX_ZERO;
            packet[header_start..header_start + 5].copy_from_slice(&[
                0x01, // Channel ID high
                0x01, // Channel ID low
                0x00, // Command tag (EAPDU)
                (sequence_number >> 8) as u8,
                (sequence_number & 0xff) as u8,
            ]);

            // Copy data
            let data_start = header_start + header_size;
            packet[data_start..data_start + chunk_size].copy_from_slice(&data[offset..offset + chunk_size]);

            // Send packet
            let written = device.write(&packet)
                .map_err(|e| KeystoneError::UsbError(format!("Failed to write packet: {}", e)))?;

            if written < chunk_size + header_size {
                return Err(KeystoneError::UsbError("Incomplete packet write".to_string()));
            }

            offset += chunk_size;
            sequence_number += 1;

            if sequence_number >= 0xffff {
                return Err(KeystoneError::UsbError("Maximum sequence number reached".to_string()));
            }
        }

        Ok(())
    }

    #[cfg(feature = "hidapi")]
    fn receive_data(&self, device: &HidDevice) -> Result<Vec<u8>, KeystoneError> {
        const HID_PACKET_SIZE: usize = 64;
        const HID_PREFIX_ZERO: usize = 0;
        const TIMEOUT_MS: i32 = 5000;

        let mut all_data = Vec::new();
        let mut sequence_number = 0;
        let mut is_first_packet = true;

        loop {
            let mut packet = [0u8; HID_PACKET_SIZE];
            let read = device.read_timeout(&mut packet, TIMEOUT_MS)
                .map_err(|e| KeystoneError::UsbError(format!("Failed to read packet: {}", e)))?;

            if read < 5 {
                return Err(KeystoneError::UsbError("Invalid packet size".to_string()));
            }

            // Parse packet header
            let header_start = HID_PREFIX_ZERO;
            let channel_id = u16::from_be_bytes([packet[header_start], packet[header_start + 1]]);
            let cmd_tag = packet[header_start + 2];
            let packet_seq = u16::from_be_bytes([packet[header_start + 3], packet[header_start + 4]]);

            if channel_id != 0x0101 || cmd_tag != 0x00 {
                return Err(KeystoneError::UsbError("Invalid packet header".to_string()));
            }

            if packet_seq != sequence_number {
                return Err(KeystoneError::UsbError("Packet sequence mismatch".to_string()));
            }

            // Extract data
            let data_start = header_start + 5;
            let data_size = read - 5;
            all_data.extend_from_slice(&packet[data_start..data_start + data_size]);

            // Check if this is the last packet
            if data_size < HID_PACKET_SIZE - 5 {
                break;
            }

            sequence_number += 1;
            is_first_packet = false;
        }

        Ok(all_data)
    }

    fn serialize_request(&self, request: &EAPDURequestPayload) -> Result<Vec<u8>, KeystoneError> {
        // Simple serialization for now - in a real implementation, this would be more sophisticated
        let mut data = Vec::new();
        
        // Header
        data.push(request.cla);
        data.extend_from_slice(&(request.command_type as u32).to_le_bytes());
        data.extend_from_slice(&request.request_id.to_le_bytes());
        
        // Data length and data
        data.extend_from_slice(&(request.data_len as u16).to_le_bytes());
        data.extend_from_slice(&request.data);

        Ok(data)
    }

    fn deserialize_response(&self, data: &[u8]) -> Result<EAPDUResponsePayload, KeystoneError> {
        if data.len() < 12 {
            return Err(KeystoneError::InvalidResponse("Response too short".to_string()));
        }

        let cla = data[0];
        let status = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);
        let request_id = u16::from_le_bytes([data[5], data[6]]);
        let command_type = u32::from_le_bytes([data[7], data[8], data[9], data[10]]);
        let data_len = u16::from_le_bytes([data[11], data[12]]);

        let response_data = if data.len() > 13 {
            data[13..13 + data_len as usize].to_vec()
        } else {
            Vec::new()
        };

        Ok(EAPDUResponsePayload {
            cla,
            data: response_data,
            data_len: data_len as u32,
            status: StatusEnum::from_u32(status),
            request_id,
            command_type: CommandType::from_u32(command_type),
        })
    }

    #[cfg(feature = "hidapi")]
    fn get_device_info_internal(&self) -> Result<RemoteWalletInfo, KeystoneError> {
        let response = self.send_apdu_command(CommandType::GetDeviceInfo, None)?;
        println!("response: {:?}", response);
        
        // Parse JSON response
        let json_str = String::from_utf8(response.data)
            .map_err(|e| KeystoneError::InvalidResponse(format!("Invalid UTF-8: {}", e)))?;
        
        let device_info: DeviceInfoResponse = serde_json::from_str(&json_str)
            .map_err(|e| KeystoneError::InvalidResponse(format!("Invalid JSON: {}", e)))?;

        Ok(RemoteWalletInfo {
            model: "Keystone Pro".to_string(),
            manufacturer: crate::locator::Manufacturer::Keystone,
            serial: device_info.walletMFP,
            host_device_path: self.device_path.clone(),
            pubkey: Pubkey::default(), // Will be populated after first get_pubkey call
            error: None,
        })
    }

    #[cfg(feature = "hidapi")]
    fn export_address(&self, derivation_path: &DerivationPath) -> Result<Pubkey, KeystoneError> {
        // Create a UR for exporting the public key
        let ur = URProtocol::create_export_pubkey_request(derivation_path)?;
        let ur_string = URProtocol::encode_ur(&ur)?;
        
        // Send the UR via the resolve UR command
        let response = self.send_apdu_command(CommandType::ResolveUr, Some(ur_string.as_bytes()))?;
        
        // Parse the response to get the public key
        let json_str = String::from_utf8(response.data)
            .map_err(|e| KeystoneError::InvalidResponse(format!("Invalid UTF-8: {}", e)))?;
        
        // Try to parse as JSON first
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) {
            if let Some(pubkey_str) = json.get("pubkey").and_then(|v| v.as_str()) {
                return Pubkey::from_str(pubkey_str)
                    .map_err(|e| KeystoneError::InvalidResponse(format!("Invalid public key: {}", e)));
            }
        }
        
        // Fallback to parsing as UR
        let ur_response = URProtocol::decode_ur(&json_str)?;
        URProtocol::parse_pubkey_response(&ur_response)
            .map_err(|e| KeystoneError::URProtocolError(e))
    }

    #[cfg(feature = "hidapi")]
    fn sign_message_with_ur(&self, derivation_path: &DerivationPath, message: &[u8]) -> Result<Signature, KeystoneError> {
        Ok(Signature::default())
        // // Create a Solana message from the raw bytes
        // let solana_message = solana_sdk::message::Message::from_bytes(message)
        //     .map_err(|e| KeystoneError::ProtocolError(format!("Invalid Solana message: {}", e)))?;
        
        // // Create a UR for the signing request
        // let ur = URProtocol::create_solana_sign_request(&solana_message, derivation_path)?;
        // let ur_string = URProtocol::encode_ur(&ur)?;
        
        // // Send the UR via the resolve UR command
        // let response = self.send_apdu_command(CommandType::ResolveUr, Some(ur_string.as_bytes()))?;
        
        // // Parse the response to get the signature
        // let json_str = String::from_utf8(response.data)
        //     .map_err(|e| KeystoneError::InvalidResponse(format!("Invalid UTF-8: {}", e)))?;
        
        // // Try to parse as UR
        // let ur_response = URProtocol::decode_ur(&json_str)?;
        // URProtocol::parse_signature_response(&ur_response)
        //     .map_err(|e| KeystoneError::URProtocolError(e))
    }
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
        #[cfg(feature = "hidapi")]
        {
            if confirm_key {
                // For Keystone, confirmation might be handled differently
                // This is a placeholder implementation
                warn!("Key confirmation not yet implemented for Keystone");
            }
            
            self.export_address(derivation_path)
                .map_err(|e| e.into())
        }

        #[cfg(not(feature = "hidapi"))]
        {
            Err(RemoteWalletError::Hid(
                "hidapi crate compilation disabled in solana-remote-wallet.".to_string(),
            ))
        }
    }

    fn sign_message(
        &self,
        derivation_path: &DerivationPath,
        data: &[u8],
    ) -> Result<Signature, RemoteWalletError> {
        #[cfg(feature = "hidapi")]
        {
            self.sign_message_with_ur(derivation_path, data)
                .map_err(|e| e.into())
        }

        #[cfg(not(feature = "hidapi"))]
        {
            Err(RemoteWalletError::Hid(
                "hidapi crate compilation disabled in solana-remote-wallet.".to_string(),
            ))
        }
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
        #[cfg(feature = "hidapi")]
        {
            self.get_device_info_internal().map_err(|e| e.into())
        }

        #[cfg(not(feature = "hidapi"))]
        {
            Ok(RemoteWalletInfo {
                model: self.model.clone(),
                manufacturer: crate::locator::Manufacturer::Keystone,
                serial: self.serial.clone(),
                host_device_path: self.device_path.clone(),
                pubkey: Pubkey::default(),
                error: None,
            })
        }
    }

    fn is_connected(&self) -> bool {
        #[cfg(feature = "hidapi")]
        {
            self.device.is_some()
        }

        #[cfg(not(feature = "hidapi"))]
        {
            false
        }
    }
}

/// Check if the detected device is a valid Keystone device
pub fn is_valid_keystone(vendor_id: u16, product_id: u16) -> bool {
    const KEYSTONE_VID: u16 = 0x1209;
    const KEYSTONE_PID: u16 = 0x3001;

    vendor_id == KEYSTONE_VID && product_id == KEYSTONE_PID
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::{
        derivation_path::DerivationPath,
        message::Message,
        system_instruction,
        pubkey::Pubkey,
    };
    use std::str::FromStr;

    #[test]
    fn test_is_valid_keystone() {
        assert!(is_valid_keystone(0x1209, 0x3001));
        assert!(!is_valid_keystone(0x1209, 0x3002));
        assert!(!is_valid_keystone(0x1208, 0x3001));
    }

    #[test]
    fn test_keystone_error_conversion() {
        let keystone_error = KeystoneError::UsbError("test error".to_string());
        let remote_error: RemoteWalletError = keystone_error.into();
        
        match remote_error {
            RemoteWalletError::Hid(_) => (),
            _ => panic!("Expected Hid error"),
        }
    }

    #[test]
    fn test_ur_protocol_integration() {
        // Test that we can create a UR for a Solana sign request
        let from_pubkey = Pubkey::new_unique();
        let to_pubkey = Pubkey::new_unique();
        let message = Message::new(
            &[system_instruction::transfer(&from_pubkey, &to_pubkey, 1000)],
            Some(&from_pubkey),
        );

        let derivation_path = DerivationPath::from_str("m/44'/501'/0'/0'").unwrap();
        let ur = URProtocol::create_solana_sign_request(&message, &derivation_path).unwrap();

        assert_eq!(ur.ur_type, "crypto-sign-request");
        assert!(!ur.cbor_data.is_empty());
    }

    #[test]
    fn test_derivation_path_parsing() {
        let path_str = "m/44'/501'/0'/0'";
        let components = URProtocol::parse_derivation_path(path_str).unwrap();
        
        assert_eq!(components.len(), 4);
        assert_eq!(components[0].index, 44);
        assert!(components[0].hardened);
        assert_eq!(components[1].index, 501);
        assert!(components[1].hardened);
    }

    #[test]
    fn test_ur_encoding_decoding() {
        let ur = UR {
            ur_type: "crypto-test".to_string(),
            cbor_data: b"test data".to_vec(),
        };

        let encoded = URProtocol::encode_ur(&ur).unwrap();
        let decoded = URProtocol::decode_ur(&encoded).unwrap();

        assert_eq!(ur.ur_type, decoded.ur_type);
        assert_eq!(ur.cbor_data, decoded.cbor_data);
    }
}
