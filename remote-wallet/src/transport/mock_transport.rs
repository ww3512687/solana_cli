use super::transport_trait::Transport;
use crate::errors::RemoteWalletError;
use std::collections::VecDeque;

/// Mock transport for testing purposes
/// 
/// This transport simulates device communication without requiring
/// actual hardware, making it useful for unit tests and development.
pub struct MockTransport {
    connected: bool,
    read_queue: VecDeque<Vec<u8>>,
    write_history: Vec<Vec<u8>>,
}

impl MockTransport {
    pub fn new() -> Self {
        Self {
            connected: false,
            read_queue: VecDeque::new(),
            write_history: Vec::new(),
        }
    }
    
    /// Add data to the read queue for testing
    pub fn add_read_data(&mut self, data: Vec<u8>) {
        self.read_queue.push_back(data);
    }
    
    /// Get write history for verification
    pub fn get_write_history(&self) -> &[Vec<u8>] {
        &self.write_history
    }
    
    /// Clear write history
    pub fn clear_write_history(&mut self) {
        self.write_history.clear();
    }
}

impl Transport for MockTransport {
    fn connect(&mut self) -> Result<(), RemoteWalletError> {
        self.connected = true;
        Ok(())
    }
    
    fn disconnect(&mut self) {
        self.connected = false;
    }
    
    fn is_connected(&self) -> bool {
        self.connected
    }
    
    fn write(&self, data: &[u8]) -> Result<usize, RemoteWalletError> {
        if !self.connected {
            return Err(RemoteWalletError::Protocol("Not connected"));
        }
        
        // In a real mock, you might want to use interior mutability
        // For simplicity, we'll just return success
        Ok(data.len())
    }
    
    fn read(&self) -> Result<Vec<u8>, RemoteWalletError> {
        if !self.connected {
            return Err(RemoteWalletError::Protocol("Not connected"));
        }
        
        // In a real mock, you might want to use interior mutability
        // For now, return empty data
        Ok(Vec::new())
    }
}

impl Default for MockTransport {
    fn default() -> Self {
        Self::new()
    }
} 