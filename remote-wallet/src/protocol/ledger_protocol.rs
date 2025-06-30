use super::common::Protocol;
use crate::errors::RemoteWalletError;
use crate::transport::common::Transport;
use std::cmp::min;

/// Ledger 协议实现
pub struct LedgerProtocol {
    transport: Box<dyn Transport>,
}

// APDU 协议常量
const APDU_TAG: u8 = 0x05;
const APDU_CLA: u8 = 0xe0;
const APDU_PAYLOAD_HEADER_LEN: usize = 7;
const DEPRECATED_APDU_PAYLOAD_HEADER_LEN: usize = 8;
const P1_NON_CONFIRM: u8 = 0x00;
const P1_CONFIRM: u8 = 0x01;
const P2_EXTEND: u8 = 0x01;
const P2_MORE: u8 = 0x02;
const MAX_CHUNK_SIZE: usize = 255;
const LEDGER_TRANSPORT_HEADER_LEN: usize = 5;
const HID_PACKET_SIZE: usize = 64;

#[cfg(windows)]
const HID_PREFIX_ZERO: usize = 1;
#[cfg(not(windows))]
const HID_PREFIX_ZERO: usize = 0;

// Ledger 命令常量
mod commands {
    pub const DEPRECATED_GET_APP_CONFIGURATION: u8 = 0x01;
    pub const DEPRECATED_GET_PUBKEY: u8 = 0x02;
    pub const DEPRECATED_SIGN_MESSAGE: u8 = 0x03;
    pub const GET_APP_CONFIGURATION: u8 = 0x04;
    pub const GET_PUBKEY: u8 = 0x05;
    pub const SIGN_MESSAGE: u8 = 0x06;
    pub const SIGN_OFFCHAIN_MESSAGE: u8 = 0x07;
}

impl LedgerProtocol {
    pub fn new(transport: Box<dyn Transport>) -> Self {
        Self { transport }
    }

    /// 发送 APDU 命令
    pub fn send_apdu(
        &self,
        command: u8,
        p1: u8,
        p2: u8,
        data: &[u8],
    ) -> Result<Vec<u8>, RemoteWalletError> {
        self._send_apdu(command, p1, p2, data, false)
    }

    /// 发送 APDU 命令（支持过时应用）
    fn _send_apdu(
        &self,
        command: u8,
        p1: u8,
        p2: u8,
        data: &[u8],
        outdated_app: bool,
    ) -> Result<Vec<u8>, RemoteWalletError> {
        self.write(command, p1, p2, data, outdated_app)?;
        
        if p1 == P1_CONFIRM && self.is_last_part(p2) {
            println!("Waiting for your approval on Ledger device");
            let result = self.read()?;
            println!("✅ Approved");
            Ok(result)
        } else {
            self.read()
        }
    }

    /// 写入 APDU 数据
    fn write(
        &self,
        command: u8,
        p1: u8,
        p2: u8,
        data: &[u8],
        outdated_app: bool,
    ) -> Result<(), RemoteWalletError> {
        let data_len = data.len();
        let mut offset = 0;
        let mut sequence_number = 0;
        let mut hid_chunk = [0_u8; HID_PACKET_SIZE];

        while sequence_number == 0 || offset < data_len {
            let header = if sequence_number == 0 {
                if outdated_app {
                    LEDGER_TRANSPORT_HEADER_LEN + DEPRECATED_APDU_PAYLOAD_HEADER_LEN
                } else {
                    LEDGER_TRANSPORT_HEADER_LEN + APDU_PAYLOAD_HEADER_LEN
                }
            } else {
                LEDGER_TRANSPORT_HEADER_LEN
            };
            
            let size = min(64 - header, data_len - offset);
            {
                let chunk = &mut hid_chunk[HID_PREFIX_ZERO..];
                chunk[0..5].copy_from_slice(&[
                    0x01,
                    0x01,
                    APDU_TAG,
                    (sequence_number >> 8) as u8,
                    (sequence_number & 0xff) as u8,
                ]);

                if sequence_number == 0 {
                    if outdated_app {
                        let data_len = data.len() + 6;
                        chunk[5..13].copy_from_slice(&[
                            (data_len >> 8) as u8,
                            (data_len & 0xff) as u8,
                            APDU_CLA,
                            command,
                            p1,
                            p2,
                            (data.len() >> 8) as u8,
                            data.len() as u8,
                        ]);
                    } else {
                        let data_len = data.len() + 5;
                        chunk[5..12].copy_from_slice(&[
                            (data_len >> 8) as u8,
                            (data_len & 0xff) as u8,
                            APDU_CLA,
                            command,
                            p1,
                            p2,
                            data.len() as u8,
                        ]);
                    }
                }

                chunk[header..header + size].copy_from_slice(&data[offset..offset + size]);
            }
            
            let n = self.transport.write(&hid_chunk[..])?;
            if n < size + header {
                return Err(RemoteWalletError::Protocol("Write data size mismatch"));
            }
            offset += size;
            sequence_number += 1;
            if sequence_number >= 0xffff {
                return Err(RemoteWalletError::Protocol("Maximum sequence number reached"));
            }
        }
        Ok(())
    }

    /// 读取 APDU 响应
    fn read(&self) -> Result<Vec<u8>, RemoteWalletError> {
        let mut message_size = 0;
        let mut message = Vec::new();

        for chunk_index in 0..=0xffff {
            let mut chunk: [u8; HID_PACKET_SIZE] = [0; HID_PACKET_SIZE];
            let chunk_size = self.transport.read(&mut chunk)?;
            
            if chunk_size < LEDGER_TRANSPORT_HEADER_LEN
                || chunk[0] != 0x01
                || chunk[1] != 0x01
                || chunk[2] != APDU_TAG
            {
                return Err(RemoteWalletError::Protocol("Unexpected chunk header"));
            }
            
            let seq = (chunk[3] as usize) << 8 | (chunk[4] as usize);
            if seq != chunk_index {
                return Err(RemoteWalletError::Protocol("Unexpected chunk header"));
            }

            let mut offset = 5;
            if seq == 0 {
                if chunk_size < 7 {
                    return Err(RemoteWalletError::Protocol("Unexpected chunk header"));
                }
                message_size = (chunk[5] as usize) << 8 | (chunk[6] as usize);
                offset += 2;
            }
            
            message.extend_from_slice(&chunk[offset..chunk_size]);
            message.truncate(message_size);
            if message.len() == message_size {
                break;
            }
        }
        
        if message.len() < 2 {
            return Err(RemoteWalletError::Protocol("No status word"));
        }
        
        let status = (message[message.len() - 2] as usize) << 8 | (message[message.len() - 1] as usize);
        Self::parse_status(status)?;
        let new_len = message.len() - 2;
        message.truncate(new_len);
        Ok(message)
    }

    /// 解析状态码
    fn parse_status(status: usize) -> Result<(), RemoteWalletError> {
        match status {
            0x9000 => Ok(()),
            0x6d00 => Err(RemoteWalletError::Protocol("Invalid instruction")),
            0x6e00 => Err(RemoteWalletError::Protocol("Invalid class")),
            0x6f00 => Err(RemoteWalletError::Protocol("Invalid parameter")),
            0x6985 => Err(RemoteWalletError::UserCancel),
            _ => Err(RemoteWalletError::Protocol("Unknown status")),
        }
    }

    /// 检查是否是最后一部分
    fn is_last_part(&self, p2: u8) -> bool {
        p2 & P2_MORE == 0
    }

    // 业务方法
    pub fn get_app_configuration(&self, outdated_app: bool) -> Result<Vec<u8>, RemoteWalletError> {
        let command = if outdated_app {
            commands::DEPRECATED_GET_APP_CONFIGURATION
        } else {
            commands::GET_APP_CONFIGURATION
        };
        self.send_apdu(command, 0, 0, &[])
    }

    pub fn get_pubkey(&self, derivation_path: &[u8], confirm_key: bool, outdated_app: bool) -> Result<Vec<u8>, RemoteWalletError> {
        let command = if outdated_app {
            commands::DEPRECATED_GET_PUBKEY
        } else {
            commands::GET_PUBKEY
        };
        let p1 = if confirm_key { P1_CONFIRM } else { P1_NON_CONFIRM };
        self.send_apdu(command, p1, 0, derivation_path)
    }

    pub fn sign_message(&self, payload: &[u8], outdated_app: bool) -> Result<Vec<u8>, RemoteWalletError> {
        let command = if outdated_app {
            commands::DEPRECATED_SIGN_MESSAGE
        } else {
            commands::SIGN_MESSAGE
        };
        self.send_apdu(command, P1_CONFIRM, 0, payload)
    }

    pub fn sign_offchain_message(&self, payload: &[u8]) -> Result<Vec<u8>, RemoteWalletError> {
        self.send_apdu(commands::SIGN_OFFCHAIN_MESSAGE, P1_CONFIRM, 0, payload)
    }
}

impl Protocol for LedgerProtocol {
    fn send_command(&self, command: &[u8]) -> Result<Vec<u8>, RemoteWalletError> {
        // 这里可以处理通用的命令发送逻辑
        self.transport.write(command)?;
        let mut buf = [0u8; 1024];
        let bytes_read = self.transport.read(&mut buf)?;
        Ok(buf[..bytes_read].to_vec())
    }

    fn transport(&self) -> &dyn Transport {
        self.transport.as_ref()
    }
} 