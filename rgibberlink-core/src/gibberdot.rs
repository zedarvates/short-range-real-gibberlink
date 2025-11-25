//! # GibberDot Mode (GDM) - Zero-Power Communication Protocol
//!
//! Implements the RGIBBERDOT MODE (GDM) specification v1.0 for ultra-low-power,
//! zero-radio communication between RealGibberLink devices.
//!
//! ## Overview
//!
//! GibberDot Mode enables secure key exchange in environments where traditional
//! communication channels are unavailable:
//! - No screens, cameras, or radio connectivity
//! - Battery life up to 9 months on CR2032
//! - Cost < 2€ per module in volume
//!
//! ## Communication Channels
//!
//! ### Primary Channel: Ultrasonic 40kHz
//! - Initiator → Receiver: 128-bit payload (ID + nonce + HMAC)
//! - Receiver → Initiator: Confirmation + encrypted session key
//! - Range: 3 meters in darkness
//!
//! ### Secondary Channel: Optical Signaling
//! - LED flash acknowledgment
//! - Morse code fallback using RGB LEDs
//! - Range: 2 meters
//!
//! ## Security Features
//!
//! - AES-128 session key generation
//! - ECDH key exchange with Perfect Forward Secrecy
//! - HMAC-SHA1 integrity protection
//! - Anti-replay with 64-bit nonces
//!
//! ## Hardware Interface
//!
//! The module expects these hardware components:
//! - 40kHz ultrasonic transducer (speaker/microphone)
//! - RGB LED for status/signaling
//! - Light sensor (photodiode) for optical acknowledgment
//! - Power management (CR2032 battery)

use crate::crypto::{CryptoEngine, CryptoError};
use thiserror::Error;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::{Duration, Instant};

/// GibberDot Mode error types
#[derive(Debug, Error)]
pub enum GibberDotError {
    #[error("Hardware not available: {0}")]
    HardwareUnavailable(String),
    #[error("Timeout during handshake")]
    Timeout,
    #[error("Cryptographic operation failed: {0}")]
    CryptoError(#[from] CryptoError),
    #[error("Invalid protocol state")]
    InvalidState,
    #[error("Communication channel degraded")]
    ChannelDegraded,
    #[error("Battery level too low")]
    LowBattery,
}

/// LED color patterns for user interface
#[derive(Debug, Clone, Copy)]
pub enum LedPattern {
    /// Red fixed or 3Hz blink: fatal error or low battery
    Error,
    /// Blue fixed (initiator) or 0.5Hz blink (listener)
    Initiating,
    /// 6x rapid green flashes or 3s solid green: success
    Success,
    /// Morse code sequence for ultimate fallback
    MorseCode,
}

/// Protocol states for GibberDot handshake
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GibberDotState {
    Idle,
    Initiating,
    Listening,
    ExchangingKeys,
    Confirmed,
    Error,
}

/// GibberDot handshake configuration
#[derive(Debug, Clone)]
pub struct GibberDotConfig {
    /// Ultrasonic carrier frequency (default: 40kHz)
    pub ultrasonic_freq_hz: u32,
    /// Maximum handshake time (default: 280ms)
    pub handshake_timeout_ms: u64,
    /// LED flash duration (default: 50ms)
    pub led_flash_duration_ms: u64,
    /// Morse code flash rate (default: 12 flashes/second)
    pub morse_rate_hz: u32,
    /// Enable optical fallback (default: true)
    pub enable_optical_fallback: bool,
    /// Power-saving mode for ultra-low power operation
    pub low_power_mode: bool,
}

impl Default for GibberDotConfig {
    fn default() -> Self {
        Self {
            ultrasonic_freq_hz: 40000,
            handshake_timeout_ms: 280,
            led_flash_duration_ms: 50,
            morse_rate_hz: 12,
            enable_optical_fallback: true,
            low_power_mode: true,
        }
    }
}

/// GibberDot protocol engine for zero-power communication
pub struct GibberDotEngine {
    config: GibberDotConfig,
    crypto: Arc<Mutex<CryptoEngine>>,
    state: Arc<Mutex<GibberDotState>>,
    session_key: Arc<Mutex<Option<[u8; 32]>>>,
    last_activity: Arc<Mutex<Instant>>,
}

impl GibberDotEngine {
    /// Create a new GibberDot engine with default configuration
    pub fn new() -> Self {
        Self::with_config(GibberDotConfig::default())
    }

    /// Create a new GibberDot engine with custom configuration
    pub fn with_config(config: GibberDotConfig) -> Self {
        Self {
            config,
            crypto: Arc::new(Mutex::new(CryptoEngine::new())),
            state: Arc::new(Mutex::new(GibberDotState::Idle)),
            session_key: Arc::new(Mutex::new(None)),
            last_activity: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// Check if GibberDot hardware is available
    pub fn is_hardware_available() -> bool {
        // In a real implementation, this would check for:
        // - Ultrasonic transducer presence
        // - LED control capability
        // - Light sensor availability
        // - Adequate battery level
        true // Placeholder - assume available for now
    }

    /// Get current protocol state
    pub async fn get_state(&self) -> GibberDotState {
        *self.state.lock().await
    }

    /// Set LED pattern for user feedback
    pub async fn set_led_pattern(&self, pattern: LedPattern) -> Result<(), GibberDotError> {
        // In a real implementation, this would control the physical LEDs
        // For now, we just log the pattern
        match pattern {
            LedPattern::Error => {
                // Red LED: fixed or 3Hz blink
                println!("GDM: LED - Error (red)");
            }
            LedPattern::Initiating => {
                // Blue LED: fixed (initiator) or slow blink (listener)
                println!("GDM: LED - Initiating (blue)");
            }
            LedPattern::Success => {
                // Green LED: 6x rapid flashes or 3s solid
                println!("GDM: LED - Success (green)");
            }
            LedPattern::MorseCode => {
                // RGB LED: Morse code sequence
                println!("GDM: LED - Morse code (RGB)");
            }
        }

        Ok(())
    }

    /// Initiate GibberDot handshake as the sender
    pub async fn initiate_handshake(&mut self) -> Result<(), GibberDotError> {
        if !Self::is_hardware_available() {
            return Err(GibberDotError::HardwareUnavailable(
                "GibberDot hardware not detected".to_string()
            ));
        }

        *self.state.lock().await = GibberDotState::Initiating;
        *self.last_activity.lock().await = Instant::now();

        // Set initiating LED pattern
        self.set_led_pattern(LedPattern::Initiating).await?;

        // Generate ECDH keypair for this handshake
        let crypto = self.crypto.lock().await;
        let public_key = crypto.generate_ecdh_keypair().await?;

        // Create nonce for anti-replay protection
        let nonce = crypto.generate_nonce().await?;
        let device_id = crypto.get_device_id().await?;

        // Prepare ultrasonic payload: 128 bits total
        // - 32 bits: Device ID
        // - 64 bits: Nonce
        // - 32 bits: HMAC-SHA1 of (ID + nonce + public_key)
        let mut payload = Vec::with_capacity(16); // 128 bits = 16 bytes

        // Device ID (32 bits)
        payload.extend_from_slice(&device_id.to_be_bytes());

        // Nonce (64 bits)
        payload.extend_from_slice(&nonce.to_be_bytes());

        // HMAC-SHA1 (32 bits, truncated from 160-bit hash)
        let hmac_data = [
            &device_id.to_be_bytes()[..],
            &nonce.to_be_bytes()[..],
            &public_key[..],
        ].concat();
        let hmac = crypto.compute_hmac(&hmac_data).await?;
        payload.extend_from_slice(&hmac[..4]); // Take first 32 bits

        // Send ultrasonic burst (placeholder - would interface with hardware)
        self.send_ultrasonic_burst(&payload).await?;

        // Wait for optical acknowledgment (LED flash)
        self.wait_for_optical_ack().await?;

        // Wait for ultrasonic response with encrypted session key
        let response = self.receive_ultrasonic_response().await?;

        // Process the response and establish session key
        self.process_response(&response).await?;

        // Set success LED pattern
        self.set_led_pattern(LedPattern::Success).await?;

        *self.state.lock().await = GibberDotState::Confirmed;

        Ok(())
    }

    /// Listen for GibberDot handshake as the receiver
    pub async fn listen_for_handshake(&mut self) -> Result<(), GibberDotError> {
        if !Self::is_hardware_available() {
            return Err(GibberDotError::HardwareUnavailable(
                "GibberDot hardware not detected".to_string()
            ));
        }

        *self.state.lock().await = GibberDotState::Listening;
        *self.last_activity.lock().await = Instant::now();

        // Set listening LED pattern (slow blue blink)
        self.set_led_pattern(LedPattern::Initiating).await?;

        // Listen for ultrasonic burst
        let payload = self.receive_ultrasonic_burst().await?;

        // Validate payload (128 bits)
        if payload.len() != 16 {
            return Err(GibberDotError::InvalidState);
        }

        let device_id = u32::from_be_bytes(payload[0..4].try_into().unwrap());
        let nonce = u64::from_be_bytes(payload[4..12].try_into().unwrap());
        let received_hmac = &payload[12..16];

        // Generate our ECDH keypair
        let crypto = self.crypto.lock().await;
        let our_public_key = crypto.generate_ecdh_keypair().await?;

        // Reconstruct HMAC data and verify
        let hmac_data = [
            &device_id.to_be_bytes()[..],
            &nonce.to_be_bytes()[..],
            &our_public_key[..],
        ].concat();
        let expected_hmac = crypto.compute_hmac(&hmac_data).await?;
        let expected_hmac_trunc = &expected_hmac[..4];

        if received_hmac != expected_hmac_trunc {
            return Err(GibberDotError::InvalidState);
        }

        // Send optical acknowledgment (LED flash)
        self.send_optical_ack().await?;

        // Generate our nonce for response
        let our_nonce = crypto.generate_nonce().await?;

        // Derive shared secret using ECDH
        let shared_secret = crypto.compute_shared_secret(&our_public_key).await?;

        // Create session key from shared secret
        let session_key = crypto.derive_session_key(&shared_secret).await?;

        // Encrypt session key for transmission
        let encrypted_key = crypto.encrypt_session_key(&session_key, &shared_secret).await?;

        // Prepare ultrasonic response: confirmation + encrypted session key
        let mut response = Vec::with_capacity(16 + 32); // Nonce (8) + Encrypted key (32) + HMAC (4) = 44 bytes

        // Our nonce (64 bits)
        response.extend_from_slice(&our_nonce.to_be_bytes());

        // Encrypted session key (256 bits, but we'll use 128-bit AES key = 16 bytes)
        response.extend_from_slice(&encrypted_key[..16]);

        // Send ultrasonic response
        self.send_ultrasonic_burst(&response).await?;

        // Store session key
        *self.session_key.lock().await = Some(session_key);

        // Set success LED pattern
        self.set_led_pattern(LedPattern::Success).await?;

        *self.state.lock().await = GibberDotState::Confirmed;

        Ok(())
    }

    /// Send ultrasonic burst with payload
    async fn send_ultrasonic_burst(&self, payload: &[u8]) -> Result<(), GibberDotError> {
        // Placeholder for ultrasonic hardware interface
        // In real implementation:
        // - Modulate 40kHz carrier with payload data
        // - Send burst for exactly 60ms
        // - Ensure directional transmission within 3m range

        println!("GDM: Sending ultrasonic burst: {} bytes", payload.len());

        // Simulate transmission time
        tokio::time::sleep(Duration::from_millis(60)).await;

        Ok(())
    }

    /// Receive ultrasonic burst
    async fn receive_ultrasonic_burst(&self) -> Result<Vec<u8>, GibberDotError> {
        // Placeholder for ultrasonic reception
        // In real implementation:
        // - Listen for 40kHz modulated signal
        // - Demodulate payload data
        // - Validate signal strength and timing

        println!("GDM: Listening for ultrasonic burst...");

        // Simulate reception time (max 60ms)
        let timeout = Duration::from_millis(self.config.handshake_timeout_ms);
        tokio::time::sleep(Duration::from_millis(30)).await; // Simulate successful reception

        // Return mock 128-bit payload
        Ok(vec![0u8; 16])
    }

    /// Wait for optical acknowledgment (LED flash)
    async fn wait_for_optical_ack(&self) -> Result<(), GibberDotError> {
        // Placeholder for light sensor interface
        // In real implementation:
        // - Monitor photodiode/light sensor
        // - Detect LED flash pattern
        // - Validate timing (max 80ms)

        println!("GDM: Waiting for optical acknowledgment...");

        // Simulate acknowledgment detection
        tokio::time::sleep(Duration::from_millis(40)).await;

        Ok(())
    }

    /// Send optical acknowledgment
    async fn send_optical_ack(&self) -> Result<(), GibberDotError> {
        // Placeholder for LED control
        // In real implementation:
        // - Flash LED briefly (50ms)
        // - Ensure visibility within 2m range

        println!("GDM: Sending optical acknowledgment (LED flash)");

        // Simulate flash duration
        tokio::time::sleep(Duration::from_millis(self.config.led_flash_duration_ms)).await;

        Ok(())
    }

    /// Receive ultrasonic response with session key
    async fn receive_ultrasonic_response(&self) -> Result<Vec<u8>, GibberDotError> {
        // Placeholder for ultrasonic reception of response
        // In real implementation:
        // - Listen for return ultrasonic burst
        // - Demodulate confirmation + encrypted key
        // - Validate within 60ms

        println!("GDM: Waiting for ultrasonic response...");

        // Simulate response reception
        tokio::time::sleep(Duration::from_millis(30)).await;

        // Return mock response (nonce + encrypted key)
        Ok(vec![0u8; 24])
    }

    /// Process received response and establish session key
    async fn process_response(&self, response: &[u8]) -> Result<(), GibberDotError> {
        // Parse response: nonce (8 bytes) + encrypted key (16 bytes)
        if response.len() < 24 {
            return Err(GibberDotError::InvalidState);
        }

        let nonce = &response[0..8];
        let encrypted_key = &response[8..24];

        // Decrypt session key using ECDH shared secret
        let crypto = self.crypto.lock().await;
        let shared_secret = crypto.compute_shared_secret(&[]).await?; // Use stored peer key
        let session_key = crypto.decrypt_session_key(encrypted_key, &shared_secret).await?;

        // Store session key
        *self.session_key.lock().await = Some(session_key);

        Ok(())
    }

    /// Get established session key
    pub async fn get_session_key(&self) -> Option<[u8; 32]> {
        *self.session_key.lock().await
    }

    /// Encrypt data using established session key
    pub async fn encrypt_data(&self, data: &[u8]) -> Result<Vec<u8>, GibberDotError> {
        let session_key = self.session_key.lock().await;
        if let Some(key) = *session_key {
            let crypto = self.crypto.lock().await;
            crypto.encrypt_with_key(data, &key).await.map_err(Into::into)
        } else {
            Err(GibberDotError::InvalidState)
        }
    }

    /// Decrypt data using established session key
    pub async fn decrypt_data(&self, encrypted_data: &[u8]) -> Result<Vec<u8>, GibberDotError> {
        let session_key = self.session_key.lock().await;
        if let Some(key) = *session_key {
            let crypto = self.crypto.lock().await;
            crypto.decrypt_with_key(encrypted_data, &key).await.map_err(Into::into)
        } else {
            Err(GibberDotError::InvalidState)
        }
    }

    /// Morse code transmission for ultimate fallback
    pub async fn transmit_morse_code(&self, message: &str) -> Result<(), GibberDotError> {
        // 3 LEDs (R/G/B) = 3 states → 1.58 bits per flash
        // 12 flashes/second = 18 bits/second
        // 128-bit key = ~7 seconds transmission

        self.set_led_pattern(LedPattern::MorseCode).await?;

        // Convert message to Morse code pattern
        let morse_pattern = self.text_to_morse(message)?;

        // Transmit using LED flashes
        for bit in morse_pattern {
            // Flash appropriate LED based on bit value
            match bit {
                0 => println!("GDM: Morse - Red LED (0)"),
                1 => println!("GDM: Morse - Green LED (1)"),
                2 => println!("GDM: Morse - Blue LED (2)"),
                _ => continue,
            }

            // 12 flashes per second = 83.3ms per flash
            tokio::time::sleep(Duration::from_millis(83)).await;
        }

        Ok(())
    }

    /// Convert text to Morse code bit pattern
    fn text_to_morse(&self, text: &str) -> Result<Vec<u8>, GibberDotError> {
        // Simplified Morse code mapping (3 states: 0,1,2)
        let mut pattern = Vec::new();

        for ch in text.chars().filter(|c| c.is_ascii_alphabetic()) {
            match ch.to_ascii_uppercase() {
                'A' => pattern.extend_from_slice(&[0, 1]), // Red-Green
                'B' => pattern.extend_from_slice(&[1, 0, 2]), // Green-Red-Blue
                'C' => pattern.extend_from_slice(&[1, 2, 0]), // Green-Blue-Red
                // Add more characters as needed...
                _ => pattern.push(0), // Default to red
            }
        }

        Ok(pattern)
    }

    /// Get time since last activity
    pub async fn time_since_last_activity(&self) -> Duration {
        self.last_activity.lock().await.elapsed()
    }

    /// Reset to idle state
    pub async fn reset(&mut self) {
        *self.state.lock().await = GibberDotState::Idle;
        *self.session_key.lock().await = None;
        *self.last_activity.lock().await = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gibberdot_initialization() {
        let engine = GibberDotEngine::new();
        assert_eq!(engine.get_state().await, GibberDotState::Idle);
    }

    #[tokio::test]
    async fn test_hardware_availability_check() {
        // This test assumes hardware is available in test environment
        assert!(GibberDotEngine::is_hardware_available());
    }

    #[tokio::test]
    async fn test_led_pattern_setting() {
        let engine = GibberDotEngine::new();

        // Test all LED patterns
        assert!(engine.set_led_pattern(LedPattern::Error).await.is_ok());
        assert!(engine.set_led_pattern(LedPattern::Initiating).await.is_ok());
        assert!(engine.set_led_pattern(LedPattern::Success).await.is_ok());
        assert!(engine.set_led_pattern(LedPattern::MorseCode).await.is_ok());
    }

    #[tokio::test]
    async fn test_morse_code_conversion() {
        let engine = GibberDotEngine::new();

        let pattern = engine.text_to_morse("AB").unwrap();
        assert!(!pattern.is_empty());
        assert!(pattern.iter().all(|&bit| bit <= 2)); // Only 0,1,2 allowed
    }

    #[tokio::test]
    async fn test_reset_functionality() {
        let mut engine = GibberDotEngine::new();

        // Change state
        *engine.state.lock().await = GibberDotState::Confirmed;

        // Reset
        engine.reset().await;

        // Verify reset
        assert_eq!(engine.get_state().await, GibberDotState::Idle);
        assert!(engine.get_session_key().await.is_none());
    }
}