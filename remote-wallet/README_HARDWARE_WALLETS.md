# Hardware Wallet Support

This document describes how to use the hardware wallet abstraction in the Solana CLI.

## Supported Hardware Wallets

The following hardware wallets are currently supported:

- **Ledger** - Full support with APDU protocol
- **Keystone** - Basic support (placeholder implementation)
- **Trezor** - Basic support (placeholder implementation)

## Architecture Overview

### HardwareWallet Trait

The `HardwareWallet` trait provides a common interface for all hardware wallets:

```rust
pub trait HardwareWallet {
    fn name(&self) -> &str;
    fn manufacturer(&self) -> &str;
    fn model(&self) -> &str;
    fn serial(&self) -> &str;
    fn device_path(&self) -> &str;
    fn firmware_version(&self) -> Result<String, RemoteWalletError>;
    fn get_pubkey(&self, derivation_path: &DerivationPath, confirm_key: bool) -> Result<Pubkey, RemoteWalletError>;
    fn sign_message(&self, derivation_path: &DerivationPath, data: &[u8]) -> Result<Signature, RemoteWalletError>;
    fn sign_offchain_message(&self, derivation_path: &DerivationPath, message: &[u8]) -> Result<Signature, RemoteWalletError>;
    fn get_device_info(&self) -> Result<RemoteWalletInfo, RemoteWalletError>;
    fn is_connected(&self) -> bool;
}
```

### RemoteWalletType Enum

The `RemoteWalletType` enum holds different wallet implementations:

```rust
pub enum RemoteWalletType {
    Ledger(Rc<LedgerWallet>),
    Keystone(Rc<KeystoneWallet>),
    Trezor(Rc<TrezorWallet>),
}
```

## Usage Examples

### Basic Usage

```rust
use solana_remote_wallet::{
    remote_wallet::RemoteWalletManager,
    hardware_wallet::HardwareWallet,
};

// Initialize wallet manager
let wallet_manager = RemoteWalletManager::new(hidapi)?;

// Update device list
wallet_manager.update_devices()?;

// List all connected devices
let devices = wallet_manager.list_devices();
for device in devices {
    println!("Found device: {} ({})", device.model, device.manufacturer);
}
```

### Working with Specific Wallet Types

```rust
// Get a specific wallet by path
let device_path = "/dev/hidraw0";
let wallet = wallet_manager.get_wallet::<LedgerWallet>(device_path)?;

// Use the wallet
let pubkey = wallet.get_pubkey(&derivation_path, false)?;
let signature = wallet.sign_message(&derivation_path, &message)?;
```

### Using the Factory Pattern

```rust
use solana_remote_wallet::wallet_factory::WalletFactory;

// Check if a manufacturer is supported
if WalletFactory::is_supported("keystone") {
    println!("Keystone is supported");
}

// Get supported manufacturers
let supported = WalletFactory::supported_manufacturers();
println!("Supported: {:?}", supported);
```

## Device Detection

The system automatically detects hardware wallets based on their vendor and product IDs:

- **Ledger**: Vendor ID `0x2c97`, various product IDs
- **Keystone**: Vendor ID `0x0483` (placeholder), various product IDs
- **Trezor**: Vendor ID `0x1209` (placeholder), various product IDs

## Error Handling

All hardware wallet operations return `Result<T, RemoteWalletError>` where `RemoteWalletError` includes:

- `DeviceNotConnected` - Device is not connected
- `DeviceLocked` - Device is locked
- `UserCancelled` - User cancelled the operation
- `InvalidDerivationPath` - Invalid derivation path
- `MessageTooLong` - Message exceeds device limits
- `UnsupportedOperation` - Operation not supported by device
- `Protocol(String)` - Protocol-specific error
- `Communication(String)` - Communication error

## Adding New Hardware Wallet Support

To add support for a new hardware wallet:

1. **Create a new wallet implementation**:
   ```rust
   pub struct NewWallet {
       // Implementation details
   }
   
   impl HardwareWallet for NewWallet {
       // Implement all required methods
   }
   ```

2. **Add device detection**:
   ```rust
   pub fn is_valid_new_wallet(vendor_id: u16, product_id: u16) -> bool {
       const NEW_WALLET_VID: u16 = 0x1234;
       const NEW_WALLET_PIDS: [u16; 2] = [0x5678, 0x5679];
       
       vendor_id == NEW_WALLET_VID && NEW_WALLET_PIDS.contains(&product_id)
   }
   ```

3. **Update the Manufacturer enum**:
   ```rust
   pub enum Manufacturer {
       Unknown,
       Ledger,
       Keystone,
       Trezor,
       NewWallet, // Add new variant
   }
   ```

4. **Update RemoteWalletType**:
   ```rust
   pub enum RemoteWalletType {
       Ledger(Rc<LedgerWallet>),
       Keystone(Rc<KeystoneWallet>),
       Trezor(Rc<TrezorWallet>),
       NewWallet(Rc<NewWallet>), // Add new variant
   }
   ```

5. **Update device detection logic** in `RemoteWalletManager::update_devices()`

6. **Add helper functions** in `wallet_factory.rs`

## Protocol Differences

Different hardware wallets use different communication protocols:

- **Ledger**: Uses APDU (Application Protocol Data Unit) over HID
- **Keystone**: Uses APDU-like protocol (to be implemented)
- **Trezor**: Uses protobuf messages over HID (to be implemented)

## Testing

To test hardware wallet support:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_wallet_detection() {
        // Test device detection logic
    }
    
    #[test]
    fn test_wallet_operations() {
        // Test wallet operations with mock devices
    }
}
```

## Future Enhancements

- [ ] Implement actual Keystone APDU communication
- [ ] Implement Trezor protobuf communication
- [ ] Add support for more hardware wallets
- [ ] Add wallet settings management
- [ ] Add firmware update support
- [ ] Add multi-signature support
- [ ] Add passphrase support 