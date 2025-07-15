use super::common::Transport;
use crate::errors::RemoteWalletError;
use std::ffi::CString;

/// HID 传输实现
pub struct HidTransport {
    #[cfg(feature = "hidapi")]
    device: Option<hidapi::HidDevice>,
    connected: bool,
}

impl HidTransport {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "hidapi")]
            device: None,
            connected: false,
        }
    }

    /// 从设备路径创建传输
    #[cfg(feature = "hidapi")]
    pub fn from_path(path: &str) -> Result<Self, RemoteWalletError> {
        let hidapi = hidapi::HidApi::new()?;
        let device = hidapi.open_path(&CString::new(path.as_bytes()).unwrap())?;
        Ok(Self {
            device: Some(device),
            connected: true,
        })
    }

    /// 从设备信息创建传输
    #[cfg(feature = "hidapi")]
    pub fn from_device_info(device_info: &hidapi::DeviceInfo) -> Result<Self, RemoteWalletError> {
        let hidapi = hidapi::HidApi::new()?;
        let device = hidapi.open_path(device_info.path())?;
        Ok(Self {
            device: Some(device),
            connected: true,
        })
    }
}

impl Transport for HidTransport {
    fn write(&self, data: &[u8]) -> Result<usize, RemoteWalletError> {
        #[cfg(feature = "hidapi")]
        {
            if let Some(ref device) = self.device {
                device.write(data)?;
                Ok(data.len())
            } else {
                Err(RemoteWalletError::NoDeviceFound)
            }
        }
        #[cfg(not(feature = "hidapi"))]
        {
            Err(RemoteWalletError::Hid("hidapi not available".to_string()))
        }
    }

    fn read(&self, buf: &mut [u8]) -> Result<usize, RemoteWalletError> {
        #[cfg(feature = "hidapi")]
        {
            if let Some(ref device) = self.device {
                let bytes_read = device.read(buf)?;
                Ok(bytes_read)
            } else {
                Err(RemoteWalletError::NoDeviceFound)
            }
        }
        #[cfg(not(feature = "hidapi"))]
        {
            Err(RemoteWalletError::Hid("hidapi not available".to_string()))
        }
    }

    fn connect(&mut self) -> Result<(), RemoteWalletError> {
        self.connected = true;
        Ok(())
    }

    fn disconnect(&mut self) {
        #[cfg(feature = "hidapi")]
        {
            self.device = None;
        }
        self.connected = false;
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}
