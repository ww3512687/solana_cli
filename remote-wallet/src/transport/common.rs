use crate::errors::RemoteWalletError;

/// 通用传输接口
pub trait Transport {
    fn write(&self, data: &[u8]) -> Result<usize, RemoteWalletError>;
    
    fn read(&self, buf: &mut [u8]) -> Result<usize, RemoteWalletError>;
    
    fn connect(&mut self) -> Result<(), RemoteWalletError>;
    
    fn disconnect(&mut self);
    
    fn is_connected(&self) -> bool;
} 