// use thiserror::Error;

// /// Wallet-specific error types
// #[derive(Error, Debug, Clone)]
// pub enum WalletError {
//     #[error("Ledger wallet error: {0}")]
//     Ledger(#[from] super::ledger::error::LedgerError),

//     #[error("Keystone wallet error: {0}")]
//     Keystone(#[from] KeystoneError),

//     #[error("Generic wallet error: {0}")]
//     Generic(String),
// }

// /// Keystone-specific error types
// #[derive(Error, Debug, Clone)]
// pub enum KeystoneError {
//     #[error("Keystone device not found")]
//     DeviceNotFound,

//     #[error("Keystone communication error: {0}")]
//     Communication(String),

//     #[error("Keystone invalid response: {0}")]
//     InvalidResponse(String),

//     #[error("Keystone operation cancelled by user")]
//     UserCancelled,

//     #[error("Keystone unsupported operation: {0}")]
//     UnsupportedOperation(String),
// }

// /// Protocol-specific error types
// #[derive(Error, Debug, Clone)]
// pub enum ProtocolError {
//     #[error("Invalid APDU command: {0}")]
//     InvalidApduCommand(String),

//     #[error("APDU response error: {0}")]
//     ApduResponse(String),

//     #[error("Protocol version mismatch: expected {expected}, got {actual}")]
//     VersionMismatch { expected: String, actual: String },

//     #[error("Protocol timeout")]
//     Timeout,
// }

// /// Transport-specific error types
// #[derive(Error, Debug, Clone)]
// pub enum TransportError {
//     #[error("HID transport error: {0}")]
//     Hid(String),

//     #[error("USB transport error: {0}")]
//     Usb(String),

//     #[error("Bluetooth transport error: {0}")]
//     Bluetooth(String),

//     #[error("Transport connection lost")]
//     ConnectionLost,

//     #[error("Transport timeout")]
//     Timeout,
// }
