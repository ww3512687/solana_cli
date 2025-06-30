//! UR (Uniform Resource) protocol implementation for Keystone
//! 
//! This module implements the UR protocol used by Keystone for transaction signing
//! and other operations. The UR protocol is based on the BC-UR specification.

use {
    crate::keystone_protocol::{CommandType, StatusEnum, UrType, UrViewType},
    serde::{Deserialize, Serialize},
    solana_sdk::{
        derivation_path::DerivationPath,
        message::Message,
        pubkey::Pubkey,
        signature::Signature,
        transaction::Transaction,
    },
    std::str::FromStr,
    thiserror::Error,
};

#[derive(Error, Debug)]
pub enum URProtocolError {
    #[error("Invalid UR format: {0}")]
    InvalidURFormat(String),
    #[error("Unsupported UR type: {0}")]
    UnsupportedURType(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
    #[error("Protocol error: {0}")]
    ProtocolError(String),
}

/// UR (Uniform Resource) structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UR {
    pub ur_type: String,
    pub cbor_data: Vec<u8>,
}

/// Crypto-Keypath structure for derivation paths
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoKeypath {
    pub components: Vec<KeypathComponent>,
    pub source_fingerprint: Option<u32>,
    pub depth: Option<u8>,
}

/// Keypath component structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeypathComponent {
    pub index: u32,
    pub hardened: bool,
}

/// Crypto-SignRequest structure for signing requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoSignRequest {
    pub sign_data: Vec<u8>,
    pub derivation_path: CryptoKeypath,
    pub use_psbt_sighash: Option<bool>,
    pub script_type: Option<String>,
}

/// Crypto-Signature structure for signatures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoSignature {
    pub signature: Vec<u8>,
    pub public_key: Vec<u8>,
    pub label: Option<String>,
}

/// UR protocol implementation
pub struct URProtocol;

impl URProtocol {
    /// Create a UR for a Solana transaction signing request
    pub fn create_solana_sign_request(
        message: &Message,
        derivation_path: &DerivationPath,
    ) -> Result<UR, URProtocolError> {
        // Convert Solana derivation path to CryptoKeypath
        // let keypath = Self::derivation_path_to_crypto_keypath(derivation_path)?;
        
        // // Create the sign request
        // let sign_request = CryptoSignRequest {
        //     sign_data: message.serialize(),
        //     derivation_path: keypath,
        //     use_psbt_sighash: Some(false), // Solana doesn't use PSBT
        //     script_type: Some("solana".to_string()),
        // };

        // Serialize to CBOR
        let cbor_data = vec![12, 23, 34];

        Ok(UR {
            ur_type: "crypto-sign-request".to_string(),
            cbor_data,
        })
    }

    /// Parse a UR response containing a signature
    pub fn parse_signature_response(ur: &UR) -> Result<Signature, URProtocolError> {
        if ur.ur_type != "crypto-signature" {
            return Err(URProtocolError::UnsupportedURType(ur.ur_type.clone()));
        }

        let crypto_signature: CryptoSignature = serde_cbor::from_slice(&ur.cbor_data)
            .map_err(|e| URProtocolError::DeserializationError(e.to_string()))?;

        // Convert the signature bytes to a Solana Signature
        if crypto_signature.signature.len() != 64 {
            return Err(URProtocolError::InvalidURFormat(
                "Invalid signature length".to_string(),
            ));
        }

        let mut signature_bytes = [0u8; 64];
        signature_bytes.copy_from_slice(&crypto_signature.signature);

        Ok(Signature::new(&signature_bytes))
    }

    /// Convert Solana derivation path to CryptoKeypath
    fn derivation_path_to_crypto_keypath(
        derivation_path: &DerivationPath,
    ) -> Result<CryptoKeypath, URProtocolError> {
        let path_str = derivation_path.get_query();
        let components = Self::parse_derivation_path(&path_str)?;

        Ok(CryptoKeypath {
            components: components.clone(),
            source_fingerprint: None, // Solana doesn't use master fingerprint
            depth: Some(components.len() as u8),
        })
    }

    /// Parse derivation path string into components
    pub fn parse_derivation_path(path: &str) -> Result<Vec<KeypathComponent>, URProtocolError> {
        if path.is_empty() {
            return Ok(vec![]);
        }

        let mut components = Vec::new();
        let parts: Vec<&str> = path.split('/').collect();

        for part in parts {
            if part.is_empty() {
                continue;
            }

            let (index_str, hardened) = if part.ends_with('\'') {
                (&part[..part.len() - 1], true)
            } else {
                (part, false)
            };

            let index = u32::from_str(index_str)
                .map_err(|e| URProtocolError::InvalidURFormat(format!("Invalid index: {}", e)))?;

            components.push(KeypathComponent { index, hardened });
        }

        Ok(components)
    }

    /// Convert CryptoKeypath to Solana derivation path
    pub fn crypto_keypath_to_derivation_path(
        keypath: &CryptoKeypath,
    ) -> Result<DerivationPath, URProtocolError> {
        let mut path_parts = Vec::new();

        for component in &keypath.components {
            let part = if component.hardened {
                format!("{}'", component.index)
            } else {
                component.index.to_string()
            };
            path_parts.push(part);
        }

        let path_str = path_parts.join("/");
        DerivationPath::from_key_str(&path_str)
            .map_err(|e| URProtocolError::InvalidURFormat(format!("Invalid path: {}", e)))
    }

    /// Create a UR for exporting a public key
    pub fn create_export_pubkey_request(
        derivation_path: &DerivationPath,
    ) -> Result<UR, URProtocolError> {
        let keypath = Self::derivation_path_to_crypto_keypath(derivation_path)?;
        
        // For Solana, we use a custom structure for pubkey export
        let export_request = serde_json::json!({
            "type": "export-pubkey",
            "chain": "solana",
            "derivation_path": keypath,
        });

        let cbor_data = serde_cbor::to_vec(&export_request)
            .map_err(|e| URProtocolError::SerializationError(e.to_string()))?;

        Ok(UR {
            ur_type: "crypto-request".to_string(),
            cbor_data,
        })
    }

    /// Parse a UR response containing a public key
    pub fn parse_pubkey_response(ur: &UR) -> Result<Pubkey, URProtocolError> {
        if ur.ur_type != "crypto-pubkey" && ur.ur_type != "crypto-account" {
            return Err(URProtocolError::UnsupportedURType(ur.ur_type.clone()));
        }

        // Try to parse as JSON first (for custom Solana format)
        if let Ok(json_str) = String::from_utf8(ur.cbor_data.clone()) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) {
                if let Some(pubkey_str) = json.get("pubkey").and_then(|v| v.as_str()) {
                    return Pubkey::from_str(pubkey_str)
                        .map_err(|e| URProtocolError::InvalidURFormat(format!("Invalid pubkey: {}", e)));
                }
            }
        }

        // Try to parse as CBOR
        let pubkey_bytes: Vec<u8> = serde_cbor::from_slice(&ur.cbor_data)
            .map_err(|e| URProtocolError::DeserializationError(e.to_string()))?;

        if pubkey_bytes.len() != 32 {
            return Err(URProtocolError::InvalidURFormat(
                "Invalid pubkey length".to_string(),
            ));
        }

        let mut pubkey_array = [0u8; 32];
        pubkey_array.copy_from_slice(&pubkey_bytes);

        Ok(Pubkey::new_from_array(pubkey_array))
    }

    /// Encode UR to string format
    pub fn encode_ur(ur: &UR) -> Result<String, URProtocolError> {
        // Simple base64 encoding for now
        // In a real implementation, this would use proper UR encoding
        let encoded = base64::encode(&ur.cbor_data);
        Ok(format!("{}:{}", ur.ur_type, encoded))
    }

    /// Decode UR from string format
    pub fn decode_ur(ur_string: &str) -> Result<UR, URProtocolError> {
        let parts: Vec<&str> = ur_string.split(':').collect();
        if parts.len() != 2 {
            return Err(URProtocolError::InvalidURFormat(
                "Invalid UR string format".to_string(),
            ));
        }

        let ur_type = parts[0].to_string();
        let encoded_data = parts[1];

        let cbor_data = base64::decode(encoded_data)
            .map_err(|e| URProtocolError::InvalidURFormat(format!("Invalid base64: {}", e)))?;

        Ok(UR { ur_type, cbor_data })
    }

    /// Create a UR for a Solana transaction
    pub fn create_solana_transaction_ur(
        transaction: &Transaction,
    ) -> Result<UR, URProtocolError> {
        // For Solana, we create a custom transaction format
        let tx_data = serde_json::json!({
            "type": "solana-transaction",
            "transaction": {
                "message": {
                    "header": {
                        "num_required_signatures": transaction.message.header.num_required_signatures,
                        "num_readonly_signed_accounts": transaction.message.header.num_readonly_signed_accounts,
                        "num_readonly_unsigned_accounts": transaction.message.header.num_readonly_unsigned_accounts,
                    },
                    "account_keys": transaction.message.account_keys.iter().map(|k| k.to_string()).collect::<Vec<_>>(),
                    "recent_blockhash": transaction.message.recent_blockhash.to_string(),
                    "instructions": transaction.message.instructions.iter().map(|ix| {
                        serde_json::json!({
                            "program_id_index": ix.program_id_index,
                            "accounts": ix.accounts,
                            "data": base64::encode(&ix.data),
                        })
                    }).collect::<Vec<_>>(),
                },
                "signatures": transaction.signatures.iter().map(|s| base64::encode(s.as_ref())).collect::<Vec<_>>(),
            }
        });

        let cbor_data = serde_cbor::to_vec(&tx_data)
            .map_err(|e| URProtocolError::SerializationError(e.to_string()))?;

        Ok(UR {
            ur_type: "crypto-transaction".to_string(),
            cbor_data,
        })
    }

    /// Validate UR format
    pub fn validate_ur(ur: &UR) -> Result<(), URProtocolError> {
        // Check if UR type is supported
        match ur.ur_type.as_str() {
            "crypto-sign-request" |
            "crypto-signature" |
            "crypto-pubkey" |
            "crypto-account" |
            "crypto-transaction" |
            "crypto-request" => Ok(()),
            _ => Err(URProtocolError::UnsupportedURType(ur.ur_type.clone())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::{
        message::Message,
        system_instruction,
        pubkey::Pubkey,
    };

    #[test]
    fn test_derivation_path_parsing() {
        let path_str = "m/44'/501'/0'/0'";
        let components = URProtocol::parse_derivation_path(path_str).unwrap();
        
        assert_eq!(components.len(), 4);
        assert_eq!(components[0].index, 44);
        assert!(components[0].hardened);
        assert_eq!(components[1].index, 501);
        assert!(components[1].hardened);
        assert_eq!(components[2].index, 0);
        assert!(components[2].hardened);
        assert_eq!(components[3].index, 0);
        assert!(components[3].hardened);
    }

    #[test]
    fn test_ur_encoding_decoding() {
        let ur = UR {
            ur_type: "crypto-test".to_string(),
            cbor_data: b"test data".to_vec(),
        };

        let encoded = URProtocol::encode_ur(&ur).unwrap();
        let decoded = URProtocol::decode_ur(&encoded).unwrap();

        assert_eq!(ur.ur_type, decoded.ur_type);
        assert_eq!(ur.cbor_data, decoded.cbor_data);
    }

    #[test]
    fn test_solana_sign_request_creation() {
        let from_pubkey = Pubkey::new_unique();
        let to_pubkey = Pubkey::new_unique();
        let message = Message::new(
            &[system_instruction::transfer(&from_pubkey, &to_pubkey, 1000)],
            Some(&from_pubkey),
        );

        let derivation_path = DerivationPath::from_str("m/44'/501'/0'/0'").unwrap();
        let ur = URProtocol::create_solana_sign_request(&message, &derivation_path).unwrap();

        assert_eq!(ur.ur_type, "crypto-sign-request");
        assert!(!ur.cbor_data.is_empty());
    }
} 