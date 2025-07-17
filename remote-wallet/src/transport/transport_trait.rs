use crate::errors::RemoteWalletError;

/// 统一的传输层 trait
/// 
/// 所有硬件钱包的通信方式都需要实现这个 trait，
/// 支持 USB HID、蓝牙、网络等多种传输方式
pub trait Transport: Send {
    /// 连接到设备
    fn connect(&mut self) -> Result<(), RemoteWalletError>;
    
    /// 断开连接
    fn disconnect(&mut self);
    
    /// 检查连接状态
    fn is_connected(&self) -> bool;
    
    /// 写入数据到设备
    fn write(&self, data: &[u8]) -> Result<usize, RemoteWalletError>;
    
    /// 从设备读取数据
    fn read(&self) -> Result<Vec<u8>, RemoteWalletError>;
} 