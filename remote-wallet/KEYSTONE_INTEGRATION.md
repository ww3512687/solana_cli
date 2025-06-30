# Keystone Hardware Wallet Integration

This document describes the integration of Keystone hardware wallet support into the Solana remote wallet library.

## Overview

The Keystone integration provides support for Keystone hardware wallets through the USB HID protocol. The implementation is based on the Keystone webusb_protocol C code and includes:

- USB HID communication with Keystone devices
- EAPDU (Extended APDU) protocol implementation
- UR (Uniform Resource) protocol for transaction signing
- Support for Solana-specific operations

## Architecture

### Core Components

1. **KeystoneWallet** (`src/keystone.rs`)
   - Main hardware wallet implementation
   - Handles USB communication and device management
   - Implements the `HardwareWallet` trait

2. **KeystoneProtocol** (`src/keystone_protocol.rs`)
   - Protocol definitions and structures
   - Command types, status codes, and data structures
   - Based on the C code from `webusb_protocol/`

3. **URProtocol** (`src/ur_protocol.rs`)
   - UR (Uniform Resource) protocol implementation
   - Handles transaction signing requests and responses
   - Supports Solana-specific message formats

### Protocol Flow

#### Device Connection
1. Device detection via USB VID/PID (0x1209/0x3001)
2. HID connection establishment
3. Device info retrieval via `CMD_GET_DEVICE_INFO`

#### Public Key Export
1. Create UR for pubkey export request
2. Send via `CMD_RESOLVE_UR` command
3. Parse response to extract public key

#### Message Signing
1. Create UR for signing request with Solana message
2. Send via `CMD_RESOLVE_UR` command
3. Wait for user approval on device
4. Parse response to extract signature

## Usage

### Basic Usage

```rust
use solana_remote_wallet::{
    keystone::KeystoneWallet,
    remote_wallet::initialize_wallet_manager,
};

// Initialize wallet manager
let wallet_manager = initialize_wallet_manager()?;

// Create Keystone wallet
let keystone = KeystoneWallet::new(device_path)?;

// Get public key
let derivation_path = DerivationPath::from_str("m/44'/501'/0'/0'")?;
let pubkey = keystone.get_pubkey(&derivation_path, false)?;

// Sign message
let signature = keystone.sign_message(&derivation_path, &message_bytes)?;
```

### CLI Integration

The Keystone wallet can be used with the Solana CLI using the USB locator format:

```bash
# Use Keystone wallet
solana config set --keypair usb://keystone

# Sign transaction
solana transfer <recipient> <amount> --keypair usb://keystone
```

## Protocol Details

### EAPDU Commands

- `CMD_GET_DEVICE_INFO` (0x00000005): Get device information
- `CMD_RESOLVE_UR` (0x00000002): Resolve UR (Uniform Resource)
- `CMD_EXPORT_ADDRESS` (0x00000004): Export address (legacy)
- `CMD_CHECK_LOCK_STATUS` (0x00000003): Check device lock status
- `CMD_ECHO_TEST` (0x00000001): Echo test command

### UR Types

- `crypto-sign-request`: Transaction signing request
- `crypto-signature`: Transaction signature response
- `crypto-pubkey`: Public key response
- `crypto-account`: Account information
- `crypto-transaction`: Transaction data

### Status Codes

- `RSP_SUCCESS_CODE` (0x00000000): Success
- `RSP_FAILURE_CODE` (0x00000001): General failure
- `PRS_EXPORT_ADDRESS_REJECTED` (0x0000000E): User rejected
- `PRS_EXPORT_ADDRESS_DISALLOWED` (0x0000000D): Device locked
- `PRS_EXPORT_ADDRESS_UNSUPPORTED_CHAIN` (0x0000000A): Unsupported chain

## Dependencies

The integration requires the following dependencies:

```toml
[dependencies]
hidapi = { workspace = true, optional = true }
serde = { workspace = true, features = ["derive"] }
serde_cbor = { workspace = true }
serde_json = { workspace = true }
base64 = { workspace = true }
```

## Testing

Run the tests with:

```bash
cargo test --package solana-remote-wallet
```

The tests include:
- Device validation
- Protocol parsing
- UR encoding/decoding
- Error handling

## Limitations

1. **Device Support**: Currently supports Keystone Pro with VID/PID 0x1209/0x3001
2. **Protocol Version**: Based on the webusb_protocol C code version
3. **Features**: Some advanced features may not be fully implemented
4. **Platform Support**: Requires HID API support (Linux, macOS, Windows)

## Future Enhancements

1. **Multi-device Support**: Support for additional Keystone models
2. **Advanced UR Features**: Full BC-UR protocol implementation
3. **Off-chain Signing**: Support for off-chain message signing
4. **Batch Operations**: Support for batch transaction signing
5. **Firmware Updates**: Support for device firmware updates

## Troubleshooting

### Common Issues

1. **Device Not Found**
   - Ensure device is connected and unlocked
   - Check USB permissions (Linux)
   - Verify VID/PID matches supported devices

2. **Communication Errors**
   - Check USB cable and connection
   - Ensure no other applications are using the device
   - Try reconnecting the device

3. **Protocol Errors**
   - Verify device firmware version
   - Check for protocol compatibility
   - Review error logs for specific issues

### Debug Logging

Enable debug logging to troubleshoot issues:

```rust
env_logger::init();
log::set_max_level(log::LevelFilter::Debug);
```

## Contributing

When contributing to the Keystone integration:

1. Follow the existing code style and patterns
2. Add tests for new functionality
3. Update documentation for API changes
4. Test with actual Keystone hardware
5. Consider backward compatibility

## References

- [Keystone Hardware Wallet](https://keyst.one/)
- [BC-UR Protocol Specification](https://github.com/BlockchainCommons/Research/blob/master/papers/bcr-2020-005-ur.md)
- [Solana Hardware Wallet Integration](https://docs.solana.com/wallet-guide/hardware-wallets)
- [USB HID Protocol](https://www.usb.org/document-library/device-class-definition-hid-111) 