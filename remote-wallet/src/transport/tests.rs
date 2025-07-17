#[cfg(test)]
mod tests {
    use super::super::transport_trait::Transport;
    use super::super::mock_transport::MockTransport;
    use crate::errors::RemoteWalletError;

    #[test]
    fn test_mock_transport_basic() {
        let mut transport = MockTransport::new();
        
        // Test initial state
        assert!(!transport.is_connected());
        
        // Test connect
        assert!(transport.connect().is_ok());
        assert!(transport.is_connected());
        
        // Test disconnect
        transport.disconnect();
        assert!(!transport.is_connected());
    }

    #[test]
    fn test_mock_transport_write_read() {
        let mut transport = MockTransport::new();
        transport.connect().unwrap();
        
        // Test write
        let data = b"test data";
        let result = transport.write(data);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), data.len());
        
        // Test read (should return empty for now)
        let result = transport.read();
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_mock_transport_not_connected() {
        let transport = MockTransport::new();
        
        // Test write without connection
        let data = b"test data";
        let result = transport.write(data);
        assert!(matches!(result, Err(RemoteWalletError::Protocol(_))));
        
        // Test read without connection
        let result = transport.read();
        assert!(matches!(result, Err(RemoteWalletError::Protocol(_))));
    }

    #[test]
    fn test_transport_trait_object() {
        let mut transport: Box<dyn Transport> = Box::new(MockTransport::new());
        
        // Test trait object works
        assert!(transport.connect().is_ok());
        assert!(transport.is_connected());
        
        let data = b"test";
        assert!(transport.write(data).is_ok());
        assert!(transport.read().is_ok());
        
        transport.disconnect();
        assert!(!transport.is_connected());
    }
} 