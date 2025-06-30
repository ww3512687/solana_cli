//! Keystone protocol definitions based on the webusb_protocol C code
//! 
//! This module contains the Rust equivalents of the C structures and enums
//! defined in the Keystone webusb_protocol directory.

/// Keystone EAPDU protocol header
pub const KEYSTONE_PROTOCOL_HEADER: u8 = 0x00;

/// Command types for Keystone EAPDU protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandType {
    EchoTest = 0x00000001,
    ResolveUr = 0x00000002,
    CheckLockStatus = 0x00000003,
    ExportAddress = 0x00000004,
    GetDeviceInfo = 0x00000005,
    GetDeviceUsbPubkey = 0x00000006,
}

impl CommandType {
    pub fn from_u32(value: u32) -> Self {
        match value {
            0x00000001 => CommandType::EchoTest,
            0x00000002 => CommandType::ResolveUr,
            0x00000003 => CommandType::CheckLockStatus,
            0x00000004 => CommandType::ExportAddress,
            0x00000005 => CommandType::GetDeviceInfo,
            0x00000006 => CommandType::GetDeviceUsbPubkey,
            _ => CommandType::EchoTest, // Default fallback
        }
    }

    pub fn as_u32(&self) -> u32 {
        match self {
            CommandType::EchoTest => 0x00000001,
            CommandType::ResolveUr => 0x00000002,
            CommandType::CheckLockStatus => 0x00000003,
            CommandType::ExportAddress => 0x00000004,
            CommandType::GetDeviceInfo => 0x00000005,
            CommandType::GetDeviceUsbPubkey => 0x00000006,
        }
    }
}

/// Status codes for Keystone EAPDU protocol responses
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusEnum {
    RspSuccessCode = 0x00000000,
    RspFailureCode = 0x00000001,
    PrsInvalidTotalPackets = 0x00000002,
    PrsInvalidIndex = 0x00000003,
    PrsParsingRejected = 0x00000004,
    PrsParsingError = 0x00000005,
    PrsParsingDisallowed = 0x00000006,
    PrsParsingUnmatched = 0x00000007,
    PrsParsingMismatchedWallet = 0x00000008,
    PrsParsingVerifyPasswordError = 0x00000009,
    PrsExportAddressUnsupportedChain = 0x0000000A,
    PrsExportAddressInvalidParams = 0x0000000B,
    PrsExportAddressError = 0x0000000C,
    PrsExportAddressDisallowed = 0x0000000D,
    PrsExportAddressRejected = 0x0000000E,
    PrsExportAddressBusy = 0x0000000F,
    PrsExportHardwareCallSuccess = 0x00000010,
}

impl StatusEnum {
    pub fn from_u32(value: u32) -> Self {
        match value {
            0x00000000 => StatusEnum::RspSuccessCode,
            0x00000001 => StatusEnum::RspFailureCode,
            0x00000002 => StatusEnum::PrsInvalidTotalPackets,
            0x00000003 => StatusEnum::PrsInvalidIndex,
            0x00000004 => StatusEnum::PrsParsingRejected,
            0x00000005 => StatusEnum::PrsParsingError,
            0x00000006 => StatusEnum::PrsParsingDisallowed,
            0x00000007 => StatusEnum::PrsParsingUnmatched,
            0x00000008 => StatusEnum::PrsParsingMismatchedWallet,
            0x00000009 => StatusEnum::PrsParsingVerifyPasswordError,
            0x0000000A => StatusEnum::PrsExportAddressUnsupportedChain,
            0x0000000B => StatusEnum::PrsExportAddressInvalidParams,
            0x0000000C => StatusEnum::PrsExportAddressError,
            0x0000000D => StatusEnum::PrsExportAddressDisallowed,
            0x0000000E => StatusEnum::PrsExportAddressRejected,
            0x0000000F => StatusEnum::PrsExportAddressBusy,
            0x00000010 => StatusEnum::PrsExportHardwareCallSuccess,
            _ => StatusEnum::RspFailureCode, // Default fallback
        }
    }

    pub fn as_u32(&self) -> u32 {
        match self {
            StatusEnum::RspSuccessCode => 0x00000000,
            StatusEnum::RspFailureCode => 0x00000001,
            StatusEnum::PrsInvalidTotalPackets => 0x00000002,
            StatusEnum::PrsInvalidIndex => 0x00000003,
            StatusEnum::PrsParsingRejected => 0x00000004,
            StatusEnum::PrsParsingError => 0x00000005,
            StatusEnum::PrsParsingDisallowed => 0x00000006,
            StatusEnum::PrsParsingUnmatched => 0x00000007,
            StatusEnum::PrsParsingMismatchedWallet => 0x00000008,
            StatusEnum::PrsParsingVerifyPasswordError => 0x00000009,
            StatusEnum::PrsExportAddressUnsupportedChain => 0x0000000A,
            StatusEnum::PrsExportAddressInvalidParams => 0x0000000B,
            StatusEnum::PrsExportAddressError => 0x0000000C,
            StatusEnum::PrsExportAddressDisallowed => 0x0000000D,
            StatusEnum::PrsExportAddressRejected => 0x0000000E,
            StatusEnum::PrsExportAddressBusy => 0x0000000F,
            StatusEnum::PrsExportHardwareCallSuccess => 0x00000010,
        }
    }
}

/// EAPDU request payload structure
#[derive(Debug, Clone)]
pub struct EAPDURequestPayload {
    pub cla: u8,
    pub data: Vec<u8>,
    pub data_len: u32,
    pub request_id: u16,
    pub command_type: CommandType,
}

/// EAPDU response payload structure
#[derive(Debug, Clone)]
pub struct EAPDUResponsePayload {
    pub cla: u8,
    pub data: Vec<u8>,
    pub data_len: u32,
    pub status: StatusEnum,
    pub request_id: u16,
    pub command_type: CommandType,
}

/// UR (Uniform Resource) types for Keystone
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UrType {
    QrHardwareCall = 0,
    QrNormalCall = 1,
}

impl UrType {
    pub fn from_u32(value: u32) -> Self {
        match value {
            0 => UrType::QrHardwareCall,
            1 => UrType::QrNormalCall,
            _ => UrType::QrNormalCall, // Default fallback
        }
    }

    pub fn as_u32(&self) -> u32 {
        match self {
            UrType::QrHardwareCall => 0,
            UrType::QrNormalCall => 1,
        }
    }
}

/// UR view types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UrViewType {
    CryptoAccount = 0,
    CryptoOutput = 1,
    CryptoPSBT = 2,
    CryptoSignRequest = 3,
    CryptoKeypath = 4,
    CryptoCoin = 5,
    CryptoSeed = 6,
    CryptoHDKey = 7,
    CryptoMultiKey = 8,
    CryptoECKey = 9,
    CryptoAddress = 10,
    CryptoSignature = 11,
    CryptoTransaction = 12,
}

impl UrViewType {
    pub fn from_u32(value: u32) -> Self {
        match value {
            0 => UrViewType::CryptoAccount,
            1 => UrViewType::CryptoOutput,
            2 => UrViewType::CryptoPSBT,
            3 => UrViewType::CryptoSignRequest,
            4 => UrViewType::CryptoKeypath,
            5 => UrViewType::CryptoCoin,
            6 => UrViewType::CryptoSeed,
            7 => UrViewType::CryptoHDKey,
            8 => UrViewType::CryptoMultiKey,
            9 => UrViewType::CryptoECKey,
            10 => UrViewType::CryptoAddress,
            11 => UrViewType::CryptoSignature,
            12 => UrViewType::CryptoTransaction,
            _ => UrViewType::CryptoAccount, // Default fallback
        }
    }

    pub fn as_u32(&self) -> u32 {
        match self {
            UrViewType::CryptoAccount => 0,
            UrViewType::CryptoOutput => 1,
            UrViewType::CryptoPSBT => 2,
            UrViewType::CryptoSignRequest => 3,
            UrViewType::CryptoKeypath => 4,
            UrViewType::CryptoCoin => 5,
            UrViewType::CryptoSeed => 6,
            UrViewType::CryptoHDKey => 7,
            UrViewType::CryptoMultiKey => 8,
            UrViewType::CryptoECKey => 9,
            UrViewType::CryptoAddress => 10,
            UrViewType::CryptoSignature => 11,
            UrViewType::CryptoTransaction => 12,
        }
    }
}

/// Constants for protocol parsing
pub const MAX_PACKETS: usize = 200;
pub const MAX_PACKETS_LENGTH: usize = 64;
pub const MAX_EAPDU_DATA_SIZE: usize = MAX_PACKETS_LENGTH - 9; // OFFSET_CDATA
pub const MAX_EAPDU_RESPONSE_DATA_SIZE: usize = MAX_PACKETS_LENGTH - 9 - 2; // OFFSET_CDATA - EAPDU_RESPONSE_STATUS_LENGTH

/// Request ID constants
pub const REQUEST_ID_IDLE: u16 = 0xFFFF;

/// Error codes
pub const ERROR_CODE_SUCCESS: u32 = 0;
pub const ERROR_CODE_FAILURE: u32 = 1;
pub const ERROR_CODE_INVALID_PARAMS: u32 = 2;
pub const ERROR_CODE_DEVICE_LOCKED: u32 = 3;
pub const ERROR_CODE_OPERATION_REJECTED: u32 = 4;
pub const ERROR_CODE_UNSUPPORTED_OPERATION: u32 = 5; 