use std::error::Error;

/// 统一硬件钱包错误 trait
pub trait HardwareWalletError: Error {
    fn code(&self) -> u16;
    fn description(&self) -> String;
}
