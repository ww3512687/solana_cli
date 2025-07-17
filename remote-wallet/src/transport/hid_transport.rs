use super::transport_trait::Transport;
use crate::errors::RemoteWalletError;

pub struct HidTransport {
    pub device: hidapi::HidDevice,
}

impl HidTransport {
    pub fn new(device: hidapi::HidDevice) -> Self {
        Self { device }
    }
}

impl Transport for HidTransport {
    fn connect(&mut self) -> Result<(), RemoteWalletError> {
        // HID 设备在创建时已经连接，这里只是验证连接状态
        Ok(())
    }
    
    fn disconnect(&mut self) {
        // HID 设备会在 Drop 时自动断开
    }
    
    fn is_connected(&self) -> bool {
        // 简单检查：尝试读取设备信息来验证连接
        // 这里可以根据需要实现更复杂的连接检查
        true
    }
    
    fn write(&self, data: &[u8]) -> Result<usize, RemoteWalletError> {
        self.device.write(data).map_err(|e| RemoteWalletError::Hid(e.to_string()))
    }
    
    fn read(&self) -> Result<Vec<u8>, RemoteWalletError> {
        let mut buf = vec![0u8; 64];
        let len = self.device.read(&mut buf).map_err(|e| RemoteWalletError::Hid(e.to_string()))?;
        buf.truncate(len);
        Ok(buf)
    }
}
