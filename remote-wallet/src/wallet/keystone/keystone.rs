use {
    super::error::KeystoneError,
    crate::{
        errors::RemoteWalletError,
        remote_wallet::{RemoteWallet, RemoteWalletInfo, RemoteWalletManager},
        wallet::{types::Device, WalletProbe},
    },
    console::Emoji,
    dialoguer::{theme::ColorfulTheme, Select},
    semver::Version as FirmwareVersion,
    solana_sdk::derivation_path::DerivationPath,
    std::{fmt, rc::Rc},
    serde_json,
};
#[cfg(feature = "hidapi")]
use {
    crate::locator::Manufacturer,
    log::*,
    num_traits::FromPrimitive,
    solana_sdk::{pubkey::Pubkey, signature::Signature},
    std::{cmp::min, convert::TryFrom},
};

static CHECK_MARK: Emoji = Emoji("✅ ", "");

const DEPRECATE_VERSION_BEFORE: FirmwareVersion = FirmwareVersion::new(0, 2, 0);

const APDU_PAYLOAD_HEADER_LEN: usize = 7;
const DEPRECATED_APDU_PAYLOAD_HEADER_LEN: usize = 8;
const P1_NON_CONFIRM: u8 = 0x00;
const P1_CONFIRM: u8 = 0x01;
const P2_EXTEND: u8 = 0x01;
const P2_MORE: u8 = 0x02;
const MAX_CHUNK_SIZE: usize = 255;

const APDU_SUCCESS_CODE: usize = 0x9000;

/// Keystone vendor ID
const KEYSTONE_VID: u16 = 0x1209;
/// Keystone product IDs
const KEYSTONE_PID: u16 = 0x3001;
const LEDGER_TRANSPORT_HEADER_LEN: usize = 5;

const HID_PACKET_SIZE: usize = 64 + HID_PREFIX_ZERO;

#[cfg(windows)]
const HID_PREFIX_ZERO: usize = 1;
#[cfg(not(windows))]
const HID_PREFIX_ZERO: usize = 0;

#[derive(Debug, Clone, Copy)]
enum CommandType {
    CMD_ECHO_TEST = 0x01,
    CMD_RESOLVE_UR = 0x02,
    CMD_CHECK_LOCK_STATUS = 0x03,
    CMD_EXPORT_ADDRESS = 0x04,
    CMD_GET_DEVICE_INFO = 0x05,
    CMD_GET_DEVICE_USB_PUBKEY = 0x06,
}

impl CommandType {
    /// Check if a u16 value corresponds to a valid CommandType
    fn is_valid_command(value: u16) -> bool {
        matches!(value, 
            0x01 | 0x02 | 0x03 | 0x04 | 0x05 | 0x06
        )
    }
    
    /// Try to convert u16 to CommandType
    fn from_u16(value: u16) -> Option<CommandType> {
        match value {
            0x01 => Some(CommandType::CMD_ECHO_TEST),
            0x02 => Some(CommandType::CMD_RESOLVE_UR),
            0x03 => Some(CommandType::CMD_CHECK_LOCK_STATUS),
            0x04 => Some(CommandType::CMD_EXPORT_ADDRESS),
            0x05 => Some(CommandType::CMD_GET_DEVICE_INFO),
            0x06 => Some(CommandType::CMD_GET_DEVICE_USB_PUBKEY),
            _ => None,
        }
    }
}

enum ConfigurationVersion {
    Deprecated(Vec<u8>),
    Current(Vec<u8>),
}

#[derive(Debug)]
pub enum PubkeyDisplayMode {
    Short,
    Long,
}

#[derive(Debug)]
pub struct LedgerSettings {
    pub enable_blind_signing: bool,
    pub pubkey_display: PubkeyDisplayMode,
}

/// Ledger Wallet device
pub struct KeystoneWallet {
    #[cfg(feature = "hidapi")]
    pub device: hidapi::HidDevice,
    pub pretty_path: String,
    pub version: FirmwareVersion,
}

impl fmt::Debug for KeystoneWallet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HidDevice")
    }
}
#[derive(Debug, Clone)]
pub struct EAPDUFrame {
    pub cla: u8,
    pub ins: CommandType,
    pub p1: u16,
    pub p2: u16,
    pub lc: u16,
    pub data: Vec<u8>,
    pub data_len: u32,
}

#[cfg(feature = "hidapi")]
impl KeystoneWallet {
    pub fn new(device: hidapi::HidDevice) -> Self {
        Self {
            device,
            pretty_path: String::default(),
            version: FirmwareVersion::new(0, 0, 0),
        }
    }

    // Transport Protocol:
    //		* Communication Channel Id		(2 bytes big endian )
    //		* Command Tag				(1 byte)
    //		* Packet Sequence ID			(2 bytes big endian)
    //		* Payload				(Optional)
    //
    // Payload
    //		* APDU Total Length			(2 bytes big endian)
    //		* APDU_CLA				(1 byte)
    //		* APDU_INS				(1 byte)
    //		* APDU_P1				(1 byte)
    //		* APDU_P2				(1 byte)
    //		* APDU_LENGTH 	        (1 byte (2 bytes DEPRECATED))
    //		* APDU_Payload				(Variable)
    //
    fn write(
        &self,
        command: CommandType,
        p1: u8,
        p2: u8,
        data: &[u8],
        data_len: usize,
    ) -> Result<(), RemoteWalletError> {
        let data_len = data.len();
        let mut offset = 0;
        let mut sequence_number = 0;
        let mut hid_chunk = [0_u8; HID_PACKET_SIZE];
        let total_packets = if data_len > 11 {
            (data_len - 11) / 64 + 1
        } else {
            1
        };
        let request_id = 0;

        while sequence_number == 0 || offset < data_len {
            let header = if sequence_number == 0 {
                LEDGER_TRANSPORT_HEADER_LEN + APDU_PAYLOAD_HEADER_LEN
            } else {
                LEDGER_TRANSPORT_HEADER_LEN
            };
            let size = min(64 - header, data_len - offset);
            {
                let chunk = &mut hid_chunk[HID_PREFIX_ZERO..];
                chunk[0..2].copy_from_slice(&[0x00, 0x00]);

                if sequence_number == 0 {
                    chunk[2..10].copy_from_slice(&[
                        (command as u16 >> 8) as u8,
                        (command as u16 & 0xff) as u8,
                        (total_packets >> 8) as u8,
                        (total_packets & 0xff) as u8,
                        (sequence_number >> 8) as u8,
                        (sequence_number & 0xff) as u8,
                        (request_id >> 8) as u8,
                        (request_id & 0xff) as u8,
                    ]);
                }

                chunk[header..header + size].copy_from_slice(&data[offset..offset + size]);
            }
            trace!("Ledger write {:?}", &hid_chunk[..]);
            println!("send hid_chunk: {:?}", &hid_chunk[..]);
            let n = self.device.write(&hid_chunk[..])?;
            if n < size + header {
                return Err(RemoteWalletError::Protocol("Write data size mismatch"));
            }
            offset += size;
            sequence_number += 1;
            if sequence_number >= 0xffff {
                return Err(RemoteWalletError::Protocol(
                    "Maximum sequence number reached",
                ));
            }
        }
        Ok(())
    }

    // Transport Protocol:
    //		* Communication Channel Id		(2 bytes big endian )
    //		* Command Tag				(1 byte)
    //		* Packet Sequence ID			(2 bytes big endian)
    //		* Payload				(Optional)
    //
    // Payload
    //		* APDU_LENGTH				(1 byte)
    //		* APDU_Payload				(Variable)
    //
    fn read(&self) -> Result<Vec<u8>, RemoteWalletError> {
        let mut buffer = [0u8; HID_PACKET_SIZE];
        let mut result_data = Vec::new();
        let mut sequence_number = 0u16;
        let mut total_length = 0usize;
        let mut received_length = 0usize;

        loop {
            // Read HID packet
            let n = self.device.read(&mut buffer)?;
            println!("n: {:?}", n);
            if n < LEDGER_TRANSPORT_HEADER_LEN {
                return Err(RemoteWalletError::Protocol("Invalid HID packet size"));
            }

            let packet = &buffer[HID_PREFIX_ZERO..n];
            println!("packet: {:?}", packet);
            
            // Parse transport header
            let cla = packet[0];
            let command = u16::from_be_bytes([packet[1], packet[2]]);
            println!("command: {:?}", command);
            let total_packets = u16::from_be_bytes([packet[3], packet[4]]);
            println!("total_packets: {:?}", total_packets);
            let packet_seq = u16::from_be_bytes([packet[5], packet[6]]);
            println!("packet_seq: {:?}", packet_seq);
            let packet_data = &packet[7..];
            println!("packet_data (hex): {:02x?}", packet_data);
            // Try to print as string if it's valid UTF-8
            if let Ok(data_str) = std::str::from_utf8(packet_data) {
                // Clean up the string by removing null characters
                let cleaned_str = data_str.trim_matches('\0').trim();
                println!("packet_data (string): {}", cleaned_str);
                
                // Try to parse as JSON if it looks like JSON
                if cleaned_str.starts_with('{') && cleaned_str.ends_with('}') {
                    match serde_json::from_str::<serde_json::Value>(cleaned_str) {
                        Ok(json) => println!("packet_data (JSON): {}", serde_json::to_string_pretty(&json).unwrap_or_default()),
                        Err(e) => println!("packet_data (JSON parse error): {}", e),
                    }
                }
            } else {
                println!("packet_data (string): <invalid UTF-8>");
            }

            // Check if command is valid
            if !CommandType::is_valid_command(command) {
                return Err(RemoteWalletError::Protocol("Invalid command"));
            }

            // Optionally, convert to CommandType enum for type safety
            let command_type = CommandType::from_u16(command)
                .ok_or(RemoteWalletError::Protocol("Invalid command type"))?;

            if packet_seq != sequence_number {
                return Err(RemoteWalletError::Protocol("Invalid packet sequence"));
            }

            if sequence_number == 0 {
            }

            sequence_number += 1;
            total_length += packet_data.len();
            result_data.extend_from_slice(packet_data);

            // Check if we have received all data
            if sequence_number == total_packets {
                break;
            }

            if sequence_number >= 0xffff {
                return Err(RemoteWalletError::Protocol("Maximum sequence number reached"));
            }
        }

        // Truncate to exact length
        result_data.truncate(total_length);
        
        // Parse status code from last 2 bytes
        if result_data.len() < 2 {
            return Err(RemoteWalletError::Protocol("Response too short"));
        }
        
        // let status_code = u16::from_be_bytes([
        //     result_data[result_data.len() - 2],
        //     result_data[result_data.len() - 1]
        // ]) as usize;
        // println!("status_code: {:?}", status_code);
        
        // Self::parse_status(status_code)?;
        
        // Remove status code from result
        result_data.truncate(result_data.len() - 2);
        
        Ok(result_data)
    }

    fn _send_apdu(
        &self,
        command: CommandType,
        p1: u8,
        p2: u8,
        data: &[u8],
        data_len: usize,
    ) -> Result<String, RemoteWalletError> {
        self.write(command, p1, p2, data, data_len)?;
        if p1 == P1_CONFIRM && is_last_part(p2) {
            println!(
                "Waiting for your approval on {} {}",
                self.name(),
                self.pretty_path
            );
            let result = self.read()?;
            println!("{CHECK_MARK}Approved");
            Ok(String::from_utf8(result).unwrap())
        } else {
            let message = self.read()?;
            println!("message: {:?}", message);
            Ok(String::from_utf8(message).unwrap())
        }
    }

    fn send_apdu(
        &self,
        command: CommandType,
        p1: u8,
        p2: u8,
        data: &[u8],
    ) -> Result<String, RemoteWalletError> {
        self._send_apdu(command, p1, p2, data, data.len())
    }

    fn get_firmware_version(&self) -> Result<FirmwareVersion, RemoteWalletError> {
        self.get_device_info().map(|config| FirmwareVersion::new(config.major, config.minor, config.patch))
    }

    // pub fn get_settings(&self) -> Result<LedgerSettings, RemoteWalletError> {
    //     self.get_device_info().map(|config| match config {
    //         ConfigurationVersion::Current(config) => {
    //             let enable_blind_signing = config[0] != 0;
    //             let pubkey_display = if config[1] == 0 {
    //                 PubkeyDisplayMode::Long
    //             } else {
    //                 PubkeyDisplayMode::Short
    //             };
    //             LedgerSettings {
    //                 enable_blind_signing,
    //                 pubkey_display,
    //             }
    //         }
    //         ConfigurationVersion::Deprecated(_) => LedgerSettings {
    //             enable_blind_signing: false,
    //             pubkey_display: PubkeyDisplayMode::Short,
    //         },
    //     })
    // }

    fn get_device_info(&self) -> Result<FirmwareVersion, RemoteWalletError> {
        let data = self._send_apdu(CommandType::CMD_GET_DEVICE_INFO, 0, 0, &[], 0)?;
        // data to json
        let json = serde_json::to_string(&data).unwrap();
        println!("json: {:?}......", json);
        let data = serde_json::from_str::<Vec<u8>>(&data).unwrap();
        Ok(FirmwareVersion::new(data[2].into(), data[3].into(), data[4].into()))
    }

    fn outdated_app(&self) -> bool {
        self.version < DEPRECATE_VERSION_BEFORE
    }

    fn parse_status(status: usize) -> Result<(), RemoteWalletError> {
        if status == APDU_SUCCESS_CODE {
            Ok(())
        } else if let Some(err) = KeystoneError::from_usize(status) {
            Err(err.into())
        } else {
            Err(RemoteWalletError::Protocol("Unknown error"))
        }
    }
}

use crate::wallet::types::RemoteWalletType;
use hidapi::{DeviceInfo, HidApi};

pub struct KeystoneProbe;
#[cfg(not(feature = "hidapi"))]
impl WalletProbe<Self> for KeystoneProbe {}
#[cfg(feature = "hidapi")]
impl WalletProbe for KeystoneProbe {
    fn is_supported_device(&self, device_info: &hidapi::DeviceInfo) -> bool {
        let data =
            device_info.product_id() == KEYSTONE_PID && device_info.vendor_id() == KEYSTONE_VID;
        if data {
            println!("data: {:?}", data);
        }
        data
    }

    fn open(&self, usb: &mut HidApi, devinfo: DeviceInfo) -> Result<Device, RemoteWalletError> {
        println!("devinfo.path(): {:?}", devinfo.path());
        let handle = usb
            .open_path(devinfo.path())
            .map_err(|e| RemoteWalletError::Hid(e.to_string()))?;
        let mut wallet = KeystoneWallet::new(handle);
        let info = wallet
            .read_device(&devinfo)
            .map_err(|e| RemoteWalletError::Hid(e.to_string()))?;
        wallet.pretty_path = info.get_pretty_path();
        println!("wallet.pretty_path: {:?}", wallet.pretty_path);
        Ok(Device {
            path: devinfo.path().to_string_lossy().into_owned(),
            info,
            wallet_type: RemoteWalletType::Keystone(Rc::new(wallet)),
        })
    }
}

#[cfg(not(feature = "hidapi"))]
impl RemoteWallet<Self> for KeystoneWallet {}
#[cfg(feature = "hidapi")]
impl RemoteWallet<hidapi::DeviceInfo> for KeystoneWallet {
    fn name(&self) -> &str {
        "Keystone hardware wallet"
    }

    fn read_device(
        &mut self,
        dev_info: &hidapi::DeviceInfo,
    ) -> Result<RemoteWalletInfo, RemoteWalletError> {
        let manufacturer = dev_info
            .manufacturer_string()
            .and_then(|s| Manufacturer::try_from(s).ok())
            .unwrap_or_default();
        let model = dev_info
            .product_string()
            .unwrap_or("Unknown")
            .to_lowercase()
            .replace(' ', "-");
        println!("{}:{:?}", file!(), line!());
        let serial = dev_info.serial_number().unwrap_or("Unknown").to_string();
        println!("{}:{:?}", file!(), line!());
        let host_device_path = dev_info.path().to_string_lossy().to_string();
        println!("{}:{:?}", file!(), line!());
        let version = self.get_firmware_version()?;
        println!("version: {:?}", version);
        println!("{}:{:?}", file!(), line!());
        self.version = version;
        println!("{}:{:?}", file!(), line!());
        let pubkey_result = self.get_pubkey(&DerivationPath::default(), false);
        println!("{}:{:?}", file!(), line!());
        let (pubkey, error) = match pubkey_result {
            Ok(pubkey) => (pubkey, None),
            Err(err) => (Pubkey::default(), Some(err)),
        };
        Ok(RemoteWalletInfo {
            model,
            manufacturer,
            serial,
            host_device_path,
            pubkey,
            error,
        })
    }

    fn get_pubkey(
        &self,
        derivation_path: &DerivationPath,
        confirm_key: bool,
    ) -> Result<Pubkey, RemoteWalletError> {
        let derivation_path = extend_and_serialize(derivation_path);

        let key = self.send_apdu(
            CommandType::CMD_RESOLVE_UR,
            if confirm_key {
                P1_CONFIRM
            } else {
                P1_NON_CONFIRM
            },
            0,
            &derivation_path,
        )?;
        let key = key.as_bytes();
        Pubkey::try_from(key).map_err(|_| RemoteWalletError::Protocol("Key packet size mismatch"))
    }

    fn sign_message(
        &self,
        derivation_path: &DerivationPath,
        data: &[u8],
    ) -> Result<Signature, RemoteWalletError> {
        // If the first byte of the data is 0xff then it is an off-chain message
        // because it starts with the Domain Specifier b"\xffsolana offchain".
        // On-chain messages, in contrast, start with either 0x80 (MESSAGE_VERSION_PREFIX)
        // or the number of signatures (0x00 - 0x13).
        if !data.is_empty() && data[0] == 0xff {
            return self.sign_offchain_message(derivation_path, data);
        }
        let mut payload = if self.outdated_app() {
            extend_and_serialize(derivation_path)
        } else {
            extend_and_serialize_multiple(&[derivation_path])
        };
        if data.len() > u16::max_value() as usize {
            return Err(RemoteWalletError::InvalidInput(
                "Message to sign is too long".to_string(),
            ));
        }

        // Check to see if this data needs to be split up and
        // sent in chunks.
        let max_size = MAX_CHUNK_SIZE - payload.len();
        let empty = vec![];
        let (data, remaining_data) = if data.len() > max_size {
            data.split_at(max_size)
        } else {
            (data, empty.as_ref())
        };

        // Pack the first chunk
        if self.outdated_app() {
            for byte in (data.len() as u16).to_be_bytes().iter() {
                payload.push(*byte);
            }
        }
        payload.extend_from_slice(data);
        trace!("Serialized payload length {:?}", payload.len());

        let p2 = if remaining_data.is_empty() {
            0
        } else {
            P2_MORE
        };

        let p1 = P1_CONFIRM;
        let mut result = self.send_apdu(CommandType::CMD_RESOLVE_UR, p1, p2, &payload)?;

        // Pack and send the remaining chunks
        if !remaining_data.is_empty() {
            let mut chunks: Vec<_> = remaining_data
                .chunks(MAX_CHUNK_SIZE)
                .map(|data| {
                    let mut payload = if self.outdated_app() {
                        (data.len() as u16).to_be_bytes().to_vec()
                    } else {
                        vec![]
                    };
                    payload.extend_from_slice(data);
                    let p2 = P2_EXTEND | P2_MORE;
                    (p2, payload)
                })
                .collect();

            // Clear the P2_MORE bit on the last item.
            chunks.last_mut().unwrap().0 &= !P2_MORE;

            for (p2, payload) in chunks {
                result = self.send_apdu(
                    CommandType::CMD_RESOLVE_UR,
                    p1,
                    p2,
                    &payload,
                )?;
            }
        }

        let result = result.as_bytes();
        println!("result: {:?}", result);

        Signature::try_from(result)
            .map_err(|_| RemoteWalletError::Protocol("Signature packet size mismatch"))
    }

    // fn sign_offchain_message(
    //     &self,
    //     derivation_path: &DerivationPath,
    //     message: &[u8],
    // ) -> Result<Signature, RemoteWalletError> {
    //     if message.len()
    //         > solana_sdk::offchain_message::v0::OffchainMessage::MAX_LEN_LEDGER
    //             + solana_sdk::offchain_message::v0::OffchainMessage::HEADER_LEN
    //     {
    //         return Err(RemoteWalletError::InvalidInput(
    //             "Off-chain message to sign is too long".to_string(),
    //         ));
    //     }

    //     let mut data = extend_and_serialize_multiple(&[derivation_path]);
    //     data.extend_from_slice(message);

    //     let p1 = P1_CONFIRM;
    //     let mut p2 = 0;
    //     let mut payload = data.as_slice();
    //     while payload.len() > MAX_CHUNK_SIZE {
    //         let chunk = &payload[..MAX_CHUNK_SIZE];
    //         self.send_apdu(CommandType::SIGN_OFFCHAIN_MESSAGE, p1, p2 | P2_MORE, chunk)?;
    //         payload = &payload[MAX_CHUNK_SIZE..];
    //         p2 |= P2_EXTEND;
    //     }

    //     let result = self.send_apdu(CommandType::SIGN_OFFCHAIN_MESSAGE, p1, p2, payload)?;
    //     Signature::try_from(result)
    //         .map_err(|_| RemoteWalletError::Protocol("Signature packet size mismatch"))
    // }
}

/// Build the derivation path byte array from a DerivationPath selection
fn extend_and_serialize(derivation_path: &DerivationPath) -> Vec<u8> {
    let byte = if derivation_path.change().is_some() {
        4
    } else if derivation_path.account().is_some() {
        3
    } else {
        2
    };
    let mut concat_derivation = vec![byte];
    for index in derivation_path.path() {
        concat_derivation.extend_from_slice(&index.to_bits().to_be_bytes());
    }
    concat_derivation
}

fn extend_and_serialize_multiple(derivation_paths: &[&DerivationPath]) -> Vec<u8> {
    let mut concat_derivation = vec![derivation_paths.len() as u8];
    for derivation_path in derivation_paths {
        concat_derivation.append(&mut extend_and_serialize(derivation_path));
    }
    concat_derivation
}

/// Choose a Ledger wallet based on matching info fields
pub fn get_keystone_from_info(
    info: RemoteWalletInfo,
    keypair_name: &str,
    wallet_manager: &RemoteWalletManager,
) -> Result<Rc<KeystoneWallet>, RemoteWalletError> {
    let devices = wallet_manager.list_devices();
    println!("devices: {:?}", devices);
    let mut matches = devices
        .iter()
        .filter(|&device_info| device_info.matches(&info));
    if matches
        .clone()
        .all(|device_info| device_info.error.is_some())
    {
        let first_device = matches.next();
        if let Some(device) = first_device {
            return Err(device.error.clone().unwrap());
        }
    }
    let mut matches: Vec<(String, String)> = matches
        .filter(|&device_info| device_info.error.is_none())
        .map(|device_info| {
            let query_item = format!("{} ({})", device_info.get_pretty_path(), device_info.model,);
            (device_info.host_device_path.clone(), query_item)
        })
        .collect();
    println!("matches: {:?}", matches);
    if matches.is_empty() {
        return Err(RemoteWalletError::NoDeviceFound);
    }
    matches.sort_by(|a, b| a.1.cmp(&b.1));
    let (host_device_paths, items): (Vec<String>, Vec<String>) = matches.into_iter().unzip();

    let wallet_host_device_path = if host_device_paths.len() > 1 {
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt(format!(
                "Multiple hardware wallets found. Please select a device for {keypair_name:?}"
            ))
            .default(0)
            .items(&items[..])
            .interact()
            .unwrap();
        &host_device_paths[selection]
    } else {
        &host_device_paths[0]
    };
    wallet_manager.get_keystone(wallet_host_device_path)
}

//
fn is_last_part(p2: u8) -> bool {
    p2 & P2_MORE == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_last_part() {
        // Bytes with bit-2 set to 0 should return true
        assert!(is_last_part(0b00));
        assert!(is_last_part(0b01));
        assert!(is_last_part(0b101));
        assert!(is_last_part(0b1001));
        assert!(is_last_part(0b1101));

        // Bytes with bit-2 set to 1 should return false
        assert!(!is_last_part(0b10));
        assert!(!is_last_part(0b11));
        assert!(!is_last_part(0b110));
        assert!(!is_last_part(0b111));
        assert!(!is_last_part(0b1010));

        // Test implementation-specific uses
        let p2 = 0;
        assert!(is_last_part(p2));
        let p2 = P2_EXTEND | P2_MORE;
        assert!(!is_last_part(p2));
        assert!(is_last_part(p2 & !P2_MORE));
    }

    #[test]
    fn test_parse_status() {
        KeystoneWallet::parse_status(APDU_SUCCESS_CODE).expect("unexpected result");
        if let RemoteWalletError::Protocol(err) = KeystoneWallet::parse_status(0x6985).unwrap_err()
        {
            assert_eq!(err, "Unknown error");
        }
        if let RemoteWalletError::Protocol(err) = KeystoneWallet::parse_status(0x6fff).unwrap_err()
        {
            assert_eq!(err, "Unknown error");
        }
    }
}
