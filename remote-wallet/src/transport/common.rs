use crate::errors::RemoteWalletError;

/// 通用传输接口
pub trait Transport {
    /// 写入数据
    fn write(&self, data: &[u8]) -> Result<usize, RemoteWalletError>;
    
    /// 读取数据
    fn read(&self, buf: &mut [u8]) -> Result<usize, RemoteWalletError>;
    
    /// 连接设备
    fn connect(&mut self) -> Result<(), RemoteWalletError>;
    
    /// 断开连接
    fn disconnect(&mut self);
    
    /// 检查连接状态
    fn is_connected(&self) -> bool;
} 