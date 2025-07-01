pub mod errors;
pub mod keystone;
pub mod ledger;

use crate::errors::RemoteWalletError;
use crate::remote_wallet::Device;
use hidapi::{DeviceInfo, HidApi};

pub trait WalletProbe {
    /// 判断这个 probe 是否支持给定设备
    fn is_supported_device(&self, device_info: &hidapi::DeviceInfo) -> bool;

    /// 打开并初始化，返回已经包装好 Rc<dyn HardwareWallet> 的 Device
    fn open(&self, usb: &mut HidApi, devinfo: DeviceInfo) -> Result<Device, RemoteWalletError>;
}
