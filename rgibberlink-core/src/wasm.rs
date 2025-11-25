//! WebAssembly bindings for RealGibber
//!
//! This module provides JavaScript-compatible bindings for browser-based
//! RealGibber applications. It exposes core cryptographic functionality,
//! protocol engine, and communication capabilities while maintaining security boundaries.

use wasm_bindgen::prelude::*;
use web_sys::console;

use crate::crypto::{CryptoEngine, CryptoError};
use crate::visual::{VisualEngine, VisualPayload};

/// WebAssembly-compatible cryptographic utilities
#[wasm_bindgen]
pub struct WasmCryptoEngine {
    inner: CryptoEngine,
}

#[wasm_bindgen]
impl WasmCryptoEngine {
    /// Create a new WebAssembly crypto engine
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmCryptoEngine {
        console::log_1(&"Initializing WebAssembly crypto engine".into());
        WasmCryptoEngine {
            inner: CryptoEngine::new(),
        }
    }

    /// Generate a random nonce
    #[wasm_bindgen]
    pub fn generate_nonce(&self) -> Result<String, JsValue> {
        let nonce = CryptoEngine::generate_nonce();
        Ok(hex::encode(nonce))
    }

    /// Generate secure random bytes
    #[wasm_bindgen]
    pub fn generate_random_bytes(&self, length: usize) -> Result<String, JsValue> {
        let bytes = CryptoEngine::generate_secure_random_bytes(length);
        Ok(hex::encode(bytes))
    }

    /// Compute HMAC
    #[wasm_bindgen]
    pub fn compute_hmac(&self, key_hex: &str, data_hex: &str) -> Result<String, JsValue> {
        let key = hex::decode(key_hex)
            .map_err(|e| JsValue::from_str(&format!("Invalid key hex: {:?}", e)))?;
        let data = hex::decode(data_hex)
            .map_err(|e| JsValue::from_str(&format!("Invalid data hex: {:?}", e)))?;

        let hmac = CryptoEngine::compute_hmac(&key, &data);
        Ok(hex::encode(hmac))
    }

    /// Verify HMAC
    #[wasm_bindgen]
    pub fn verify_hmac(&self, key_hex: &str, data_hex: &str, hmac_hex: &str) -> Result<bool, JsValue> {
        let key = hex::decode(key_hex)
            .map_err(|e| JsValue::from_str(&format!("Invalid key hex: {:?}", e)))?;
        let data = hex::decode(data_hex)
            .map_err(|e| JsValue::from_str(&format!("Invalid data hex: {:?}", e)))?;
        let hmac = hex::decode(hmac_hex)
            .map_err(|e| JsValue::from_str(&format!("Invalid HMAC hex: {:?}", e)))?;

        match CryptoEngine::verify_hmac(&key, &data, &hmac) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Encrypt data with AES-GCM
    #[wasm_bindgen]
    pub fn encrypt_data(&self, key_hex: &str, data: &str) -> Result<String, JsValue> {
        let key = hex::decode(key_hex)
            .map_err(|e| JsValue::from_str(&format!("Invalid key hex: {:?}", e)))?;
        let data_bytes = data.as_bytes();

        let encrypted = CryptoEngine::encrypt_data(&key, data_bytes)
            .map_err(|e| JsValue::from_str(&format!("Encryption failed: {:?}", e)))?;

        Ok(hex::encode(encrypted))
    }

    /// Decrypt data with AES-GCM
    #[wasm_bindgen]
    pub fn decrypt_data(&self, key_hex: &str, encrypted_hex: &str) -> Result<String, JsValue> {
        let key = hex::decode(key_hex)
            .map_err(|e| JsValue::from_str(&format!("Invalid key hex: {:?}", e)))?;
        let encrypted = hex::decode(encrypted_hex)
            .map_err(|e| JsValue::from_str(&format!("Invalid encrypted hex: {:?}", e)))?;

        let decrypted = CryptoEngine::decrypt_data(&key, &encrypted)
            .map_err(|e| JsValue::from_str(&format!("Decryption failed: {:?}", e)))?;

        String::from_utf8(decrypted)
            .map_err(|e| JsValue::from_str(&format!("UTF-8 decode failed: {:?}", e)))
    }
}

/// WebAssembly-compatible visual engine for QR codes
#[wasm_bindgen]
pub struct WasmVisualEngine {
    inner: VisualEngine,
}

#[wasm_bindgen]
impl WasmVisualEngine {
    /// Create a new WebAssembly visual engine
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmVisualEngine {
        console::log_1(&"Initializing WebAssembly visual engine".into());
        WasmVisualEngine {
            inner: VisualEngine::new(),
        }
    }

    /// Generate QR code from payload
    #[wasm_bindgen]
    pub fn encode_payload(&self, session_id_hex: &str, public_key_hex: &str, nonce_hex: &str) -> Result<String, JsValue> {
        let session_id = hex::decode(session_id_hex)
            .map_err(|e| JsValue::from_str(&format!("Invalid session ID: {:?}", e)))?;
        let session_id_array: [u8; 16] = session_id.try_into()
            .map_err(|_| JsValue::from_str("Session ID must be 16 bytes"))?;

        let public_key = hex::decode(public_key_hex)
            .map_err(|e| JsValue::from_str(&format!("Invalid public key: {:?}", e)))?;

        let nonce = hex::decode(nonce_hex)
            .map_err(|e| JsValue::from_str(&format!("Invalid nonce: {:?}", e)))?;
        let nonce_array: [u8; 16] = nonce.try_into()
            .map_err(|_| JsValue::from_str("Nonce must be 16 bytes"))?;

        let payload = VisualPayload {
            session_id: session_id_array,
            public_key,
            nonce: nonce_array,
            signature: Vec::new(), // Simplified for WebAssembly
        };

        self.inner.encode_payload(&payload)
            .map_err(|e| JsValue::from_str(&format!("QR encoding failed: {:?}", e)))
    }

    /// Decode QR code payload
    #[wasm_bindgen]
    pub fn decode_payload(&self, qr_data: &str) -> Result<String, JsValue> {
        let qr_bytes = qr_data.as_bytes();
        let payload = self.inner.decode_payload(qr_bytes)
            .map_err(|e| JsValue::from_str(&format!("QR decoding failed: {:?}", e)))?;

        let result = serde_json::json!({
            "session_id": hex::encode(payload.session_id),
            "public_key": hex::encode(payload.public_key),
            "nonce": hex::encode(payload.nonce),
            "signature": hex::encode(payload.signature),
        });

        serde_json::to_string(&result)
            .map_err(|e| JsValue::from_str(&format!("JSON serialization failed: {:?}", e)))
    }
}

/// Simplified WebAssembly-compatible protocol engine for browser demo
/// This is a synchronous version that simulates protocol operations for demonstration
#[wasm_bindgen]
pub struct WasmProtocolEngine {
    crypto: CryptoEngine,
    visual: VisualEngine,
    session_id: [u8; 16],
    state: String,
    mode: String,
}

#[wasm_bindgen]
impl WasmProtocolEngine {
    /// Create a new WebAssembly protocol engine
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmProtocolEngine {
        console::log_1(&"Initializing WebAssembly protocol engine".into());

        let mut session_id = [0u8; 16];
        getrandom::getrandom(&mut session_id).unwrap_or_default();

        WasmProtocolEngine {
            crypto: CryptoEngine::new(),
            visual: VisualEngine::new(),
            session_id,
            state: "idle".to_string(),
            mode: "short-range".to_string(),
        }
    }

    /// Get current protocol state
    #[wasm_bindgen]
    pub fn get_state(&self) -> String {
        self.state.clone()
    }

    /// Get communication mode
    #[wasm_bindgen]
    pub fn get_mode(&self) -> String {
        self.mode.clone()
    }

    /// Set communication mode
    #[wasm_bindgen]
    pub fn set_mode(&mut self, mode: &str) -> Result<(), JsValue> {
        match mode {
            "short-range" | "long-range" | "auto" => {
                self.mode = mode.to_string();
                Ok(())
            }
            _ => Err(JsValue::from_str("Invalid communication mode")),
        }
    }

    /// Initiate handshake (short-range)
    #[wasm_bindgen]
    pub fn initiate_handshake(&mut self) -> Result<(), JsValue> {
        if self.state != "idle" {
            return Err(JsValue::from_str("Invalid state for handshake"));
        }
        self.state = "waiting_for_qr".to_string();
        Ok(())
    }

    /// Receive nonce and generate QR code
    #[wasm_bindgen]
    pub fn receive_nonce(&self, nonce_hex: &str) -> Result<String, JsValue> {
        let nonce = hex::decode(nonce_hex)
            .map_err(|e| JsValue::from_str(&format!("Invalid nonce hex: {:?}", e)))?;

        let nonce_array: [u8; 16] = nonce.try_into()
            .map_err(|_| JsValue::from_str("Nonce must be 16 bytes"))?;

        // Create visual payload
        let payload = VisualPayload {
            session_id: self.session_id,
            public_key: self.crypto.public_key().to_vec(),
            nonce: nonce_array,
            signature: vec![], // Simplified for demo
        };

        let qr_svg = self.visual.encode_payload(&payload)
            .map_err(|e| JsValue::from_str(&format!("QR encoding failed: {:?}", e)))?;

        Ok(qr_svg)
    }

    /// Process QR code payload
    #[wasm_bindgen]
    pub fn process_qr_payload(&mut self, qr_data: &str) -> Result<(), JsValue> {
        if self.state != "waiting_for_qr" {
            return Err(JsValue::from_str("Not waiting for QR"));
        }

        let qr_bytes = qr_data.as_bytes();
        let _payload = self.visual.decode_payload(qr_bytes)
            .map_err(|e| JsValue::from_str(&format!("QR decoding failed: {:?}", e)))?;

        // In a real implementation, we'd store the peer key and derive shared secret
        self.state = "connected".to_string();
        Ok(())
    }

    /// Receive ACK
    #[wasm_bindgen]
    pub fn receive_ack(&mut self) -> Result<(), JsValue> {
        if self.state != "waiting_for_qr" {
            return Err(JsValue::from_str("Not waiting for ACK"));
        }
        self.state = "connected".to_string();
        Ok(())
    }

    /// Send text message (encrypt and return hex)
    #[wasm_bindgen]
    pub fn send_text_message(&self, message: &str) -> Result<String, JsValue> {
        if self.state != "connected" {
            return Err(JsValue::from_str("Not connected"));
        }

        // For demo, use a dummy key. In real implementation, use shared secret
        let dummy_key = [1u8; 32];
        let encrypted = CryptoEngine::encrypt_data(&dummy_key, message.as_bytes())
            .map_err(|e| JsValue::from_str(&format!("Encryption failed: {:?}", e)))?;

        Ok(hex::encode(encrypted))
    }

    /// Receive and decrypt message
    #[wasm_bindgen]
    pub fn receive_message(&self, encrypted_hex: &str) -> Result<String, JsValue> {
        if self.state != "connected" {
            return Err(JsValue::from_str("Not connected"));
        }

        let encrypted = hex::decode(encrypted_hex)
            .map_err(|e| JsValue::from_str(&format!("Invalid encrypted hex: {:?}", e)))?;

        // For demo, use a dummy key. In real implementation, use shared secret
        let dummy_key = [1u8; 32];
        let decrypted = CryptoEngine::decrypt_data(&dummy_key, &encrypted)
            .map_err(|e| JsValue::from_str(&format!("Decryption failed: {:?}", e)))?;

        String::from_utf8(decrypted)
            .map_err(|e| JsValue::from_str(&format!("UTF-8 decode failed: {:?}", e)))
    }

    /// Get pending messages (for demo purposes)
    #[wasm_bindgen]
    pub fn get_pending_messages(&self) -> String {
        "[]".to_string()
    }

    /// Get session ID
    #[wasm_bindgen]
    pub fn get_session_id(&self) -> String {
        hex::encode(self.session_id)
    }

    /// Get public key
    #[wasm_bindgen]
    pub fn get_public_key(&self) -> String {
        hex::encode(self.crypto.public_key())
    }

    /// Generate nonce
    #[wasm_bindgen]
    pub fn generate_nonce(&self) -> String {
        hex::encode(CryptoEngine::generate_nonce())
    }

    /// Generate random bytes
    #[wasm_bindgen]
    pub fn generate_random_bytes(&self, length: usize) -> String {
        hex::encode(CryptoEngine::generate_secure_random_bytes(length))
    }
}

/// Initialize WebAssembly module
#[wasm_bindgen(start)]
pub fn main() {
    console::log_1(&"RealGibber WebAssembly module initialized".into());
}

/// Export functions for JavaScript
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// Macro for easy logging
macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to RealGibber WebAssembly", name)
}

#[wasm_bindgen]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[wasm_bindgen]
pub fn get_build_info() -> String {
    format!(
        "RealGibber WebAssembly v{} - Secure Directional Communication",
        env!("CARGO_PKG_VERSION")
    )
}
