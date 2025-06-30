use crate::errors::RemoteWalletError;
use crate::transport::common::Transport;

/// 通用协议接口
pub trait Protocol {
    /// 发送命令并获取响应
    fn send_command(&self, command: &[u8]) -> Result<Vec<u8>, RemoteWalletError>;
    
    /// 获取传输层
    fn transport(&self) -> &dyn Transport;
} 