use crate::api::hardware_wallet::{HardwareWallet, RemoteWalletInfo};
use crate::errors::RemoteWalletError;
use crate::protocol::ledger_protocol::LedgerProtocol;
use crate::transport::hid_transport::HidTransport;
use crate::transport::common::Transport;
use semver::Version as FirmwareVersion;
use solana_sdk::{
    derivation_path::DerivationPath,
    pubkey::Pubkey,
    signature::Signature,
};
use std::rc::Rc;

/// 新的 Ledger 钱包实现
pub struct LedgerWallet {
    protocol: Rc<LedgerProtocol>,
    pretty_path: String,
    version: FirmwareVersion,
}

impl LedgerWallet {
    /// 从 HID 设备信息创建钱包
    #[cfg(feature = "hidapi")]
    pub fn new(device_info: &hidapi::DeviceInfo) -> Result<Self, RemoteWalletError> {
        let transport = HidTransport::from_device_info(device_info)?;
        let protocol = Rc::new(LedgerProtocol::new(Box::new(transport)));
        
        Ok(Self {
            protocol,
            pretty_path: device_info.path().to_string_lossy().to_string(),
            version: FirmwareVersion::new(0, 0, 0),
        })
    }

    /// 从现有传输层创建钱包
    pub fn from_transport(transport: Box<dyn Transport>, pretty_path: String) -> Self {
        let protocol = Rc::new(LedgerProtocol::new(transport));
        Self {
            protocol,
            pretty_path,
            version: FirmwareVersion::new(0, 0, 0),
        }
    }

    /// 获取固件版本
    pub fn get_firmware_version(&self) -> Result<FirmwareVersion, RemoteWalletError> {
        // TODO: 实现固件版本获取
        Ok(self.version.clone())
    }

    /// 检查是否是过时的应用
    fn outdated_app(&self) -> bool {
        self.version < FirmwareVersion::new(0, 2, 0)
    }

    /// 扩展并序列化派生路径
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

    /// 扩展并序列化多个派生路径
    fn extend_and_serialize_multiple(derivation_paths: &[&DerivationPath]) -> Vec<u8> {
        let mut data = vec![derivation_paths.len() as u8];
        for derivation_path in derivation_paths {
            data.extend_from_slice(&Self::extend_and_serialize(derivation_path));
        }
        data
    }
}

impl HardwareWallet for LedgerWallet {
    fn name(&self) -> &str {
        "Ledger hardware wallet"
    }

    fn read_device(&mut self) -> Result<RemoteWalletInfo, RemoteWalletError> {
        // TODO: 实现设备信息读取
        Ok(RemoteWalletInfo {
            model: "ledger".to_string(),
            manufacturer: crate::locator::Manufacturer::Ledger,
            serial: "".to_string(),
            host_device_path: self.pretty_path.clone(),
            pubkey: Default::default(),
            error: None,
        })
    }

    fn get_pubkey(&self, derivation_path: &DerivationPath, confirm_key: bool) -> Result<Pubkey, RemoteWalletError> {
        let derivation_data = Self::extend_and_serialize(derivation_path);
        let outdated_app = self.outdated_app();
        
        let key_data = self.protocol.get_pubkey(&derivation_data, confirm_key, outdated_app)?;
        Pubkey::try_from(key_data.as_slice())
            .map_err(|_| RemoteWalletError::Protocol("Key packet size mismatch"))
    }

    fn sign_message(&self, derivation_path: &DerivationPath, data: &[u8]) -> Result<Signature, RemoteWalletError> {
        // 检查是否是离线消息
        if !data.is_empty() && data[0] == 0xff {
            return self.sign_offchain_message(derivation_path, data);
        }

        let outdated_app = self.outdated_app();
        let mut payload = if outdated_app {
            Self::extend_and_serialize(derivation_path)
        } else {
            Self::extend_and_serialize_multiple(&[derivation_path])
        };

        if data.len() > u16::max_value() as usize {
            return Err(RemoteWalletError::InvalidInput("Message to sign is too long".to_string()));
        }

        // 添加数据长度（过时应用）
        if outdated_app {
            payload.extend_from_slice(&(data.len() as u16).to_be_bytes());
        }
        
        payload.extend_from_slice(data);

        let result = self.protocol.sign_message(&payload, outdated_app)?;
        Signature::try_from(result.as_slice())
            .map_err(|_| RemoteWalletError::Protocol("Signature packet size mismatch"))
    }

    fn sign_offchain_message(&self, derivation_path: &DerivationPath, message: &[u8]) -> Result<Signature, RemoteWalletError> {
        if message.len() > solana_sdk::offchain_message::v0::OffchainMessage::MAX_LEN_LEDGER
            + solana_sdk::offchain_message::v0::OffchainMessage::HEADER_LEN
        {
            return Err(RemoteWalletError::InvalidInput("Off-chain message to sign is too long".to_string()));
        }

        let mut payload = Self::extend_and_serialize_multiple(&[derivation_path]);
        payload.extend_from_slice(message);

        let result = self.protocol.sign_offchain_message(&payload)?;
        Signature::try_from(result.as_slice())
            .map_err(|_| RemoteWalletError::Protocol("Signature packet size mismatch"))
    }
}
