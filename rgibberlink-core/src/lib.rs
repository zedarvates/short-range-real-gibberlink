//! # RgibberLink Core Library
//!
//! A comprehensive Rust library implementing both short-range and long-range secure directional
//! communication protocols. Supports ultrasonic audio transmission (18-22kHz FSK) with GGWave
//! integration, ECDH key exchange, QR codes with Reed-Solomon ECC, and advanced long-range
//! capabilities using focused ultrasound beams and laser modulation.
//!
//! ## Architecture Overview
//!
//! The library consists of modular components designed for both short-range pairing and
//! long-range directional communication:
//!
//! ### Core Engines
//! - **`CryptoEngine`**: Handles ECDH key exchange, AES-GCM encryption, HMAC verification, and cross-channel signatures
//! - **`AudioEngine`**: Manages GGWave ultrasonic transmissions for short-range communication
//! - **`UltrasonicBeamEngine`**: Focused ultrasound communication (10-30m range) with parametric audio and beam forming
//! - **`VisualEngine`**: Generates QR codes with Reed-Solomon ECC and CBOR compression
//! - **`LaserEngine`**: High-speed optical data transmission with laser modulation, OOK/PWM/QM schemes, and adaptive ECC
//! - **`RangeDetector`**: Ultrasonic time-of-flight ranging for distance measurement and power optimization
//! - **`OpticalECC`**: Advanced error correction for laser transmission with atmospheric compensation
//! - **`ChannelValidator`**: Implements coupled channel validation requiring simultaneous presence in both beams
//! - **`ProtocolEngine`**: Implements the handshake state machine with coupled validation and fallback mechanisms
//! - **`SecurityManager`**: Permission-based access control with peer trust assessment and environmental monitoring
//! - **`FallbackManager`**: Automatic degradation from long-range to short-range modes with recovery monitoring
//!
//! ## Communication Modes
//!
//! ### Short-Range Mode (0-5m)
//! Traditional pairing protocol using ultrasonic audio and visual QR codes:
//! 1. Device A sends nonce via ultrasonic audio (18-22kHz FSK)
//! 2. Device B displays QR code with public key, session ID, and nonce
//! 3. Device A scans QR code and derives shared secret via ECDH
//! 4. Device A sends ACK via ultrasonic audio
//! 5. Secure connection established with AES-GCM encryption
//!
//! ### Long-Range Mode (10-200m)
//! Advanced directional communication using coupled laser and ultrasound channels:
//! 1. Device A sends sync pulse + nonce via focused ultrasound beam
//! 2. Device B detects presence and responds with laser-modulated key data (OOK/PWM/QM)
//! 3. Device A receives laser data and validates coupled channel authentication
//! 4. Device A sends ACK via ultrasound with cross-channel signature verification
//! 5. Secure connection established with adaptive ECC and continuous monitoring
//!
//! ### Auto Mode
//! Intelligent mode selection based on hardware availability and environmental conditions:
//! - Detects available hardware (laser, parametric ultrasound transducers)
//! - Monitors channel quality and automatically switches between modes
//! - Provides seamless fallback when long-range channels degrade
//!
//! ## Long-Range Architecture Details
//!
//! ### Hybrid Channel System
//! The long-range system combines two complementary directional channels:
//!
//! #### Ultrasound Channel (Control/Auth)
//! - **Technology**: Parametric transducers with 40kHz carrier frequency
//! - **Range**: 10-30 meters in direct line-of-sight
//! - **Role**: Synchronization, authentication, presence detection, control signals
//! - **Advantages**: Inaudible, highly directional, difficult to intercept outside beam
//! - **Bandwidth**: Low (control channel) with high reliability
//!
//! #### Laser Channel (Data)
//! - **Technology**: Modulated visible/IR laser diode with beam steering
//! - **Range**: 50-200 meters with optical alignment
//! - **Modulation**: OOK, PWM, QR projection, frequency modulation
//! - **Role**: High-speed data transmission with error correction
//! - **Advantages**: High bandwidth (1-10 Mbps), directional, hard to intercept
//! - **Bandwidth**: High-speed data with Reed-Solomon + convolutional ECC
//!
//! ### Coupled Channel Security
//! Requires simultaneous reception from both channels for authentication:
//! - **Temporal Coupling**: Channels must arrive within 100ms window
//! - **Cross-Channel Signatures**: Each channel authenticates the other
//! - **Anti-Replay Protection**: Coupled nonces prevent replay attacks
//! - **Quality Validation**: Minimum signal strength and correlation thresholds
//!
//! ### Adaptive Features
//! - **Range Detection**: Ultrasonic time-of-flight for distance measurement
//! - **Power Profiles**: Adaptive laser power based on measured range (10-200m)
//! - **Modulation Selection**: Automatic scheme selection (OOK/PWM/QR) based on conditions
//! - **ECC Adaptation**: Weather/environmental compensation with optical ECC
//! - **Beam Alignment**: Camera-assisted laser alignment with continuous tracking
//!
//! ## Hardware Requirements
//!
//! ### Long-Range Transmitter
//! - Parametric ultrasonic transducer (40kHz carrier)
//! - Laser diode module (visible/IR) with modulation capability
//! - Beam steering system (optional servo/camera)
//! - Power management for adaptive output (1-100mW)
//! - Android-compatible audio interfaces
//!
//! ### Long-Range Receiver
//! - Focused ultrasound microphone/hydrophone
//! - Photodiode or camera for laser reception
//! - Signal processing for demodulation
//! - Android hardware acceleration support
//!
//! ## Performance Characteristics
//!
//! ### Short-Range Mode
//! - **Handshake Time**: 100-300ms end-to-end
//! - **Range**: 0-5 meters
//! - **Bandwidth**: ~1 kbps (GGWave limited)
//! - **Security**: ECDH + AES-GCM + anti-replay
//! - **Reliability**: High (indoor environments)
//!
//! ### Long-Range Mode
//! - **Handshake Time**: 200-500ms with coupling validation
//! - **Range**: 10-200 meters (line-of-sight)
//! - **Bandwidth**: 1-10 Mbps (laser channel)
//! - **Security**: Coupled channels + cross-signatures + adaptive ECC
//! - **Reliability**: Weather-dependent with automatic fallback
//!
//! ## Example Usage
//!
//! ### Basic Short-Range Handshake
//! ```rust
//! use gibberlink_core::RgibberLink;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut link = RgibberLink::new();
//!
//! // Initiate short-range handshake
//! link.initiate_handshake().await?;
//!
//! // Wait for QR code scan and connection
//! // ... handshake completes ...
//!
//! // Send encrypted message
//! let encrypted = link.encrypt_message(b"Hello, World!").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Long-Range Handshake with Hardware Setup
//! ```rust
//! use gibberlink_core::{RgibberLink, UltrasonicBeamEngine, LaserEngine, RangeDetector};
//!
//! # async fn long_range_example() -> Result<(), Box<dyn std::error::Error>> {
//! // Initialize long-range engines
//! let mut beam_engine = UltrasonicBeamEngine::new();
//! beam_engine.initialize().await?;
//!
//! let laser_config = gibberlink_core::LaserConfig {
//!     laser_type: gibberlink_core::LaserType::Visible,
//!     modulation: gibberlink_core::ModulationScheme::Ook,
//!     max_power_mw: 50.0,
//!     range_meters: 100.0,
//!     data_rate_bps: 1_000_000,
//!     ..Default::default()
//! };
//! let mut laser_engine = LaserEngine::new(laser_config, Default::default());
//! laser_engine.initialize().await?;
//!
//! // Enable adaptive ranging
//! let range_detector = RangeDetector::new();
//! laser_engine.enable_adaptive_mode(std::sync::Arc::new(tokio::sync::Mutex::new(range_detector)));
//!
//! // Start handshake (long-range mode will be auto-detected)
//! let mut link = RgibberLink::new();
//! link.initiate_handshake().await?;
//!
//! // Engines handle coupled channel communication
//! // ... handshake completes with validation ...
//!
//! // Send high-bandwidth data via laser channel
//! let encrypted = link.encrypt_message(b"High-speed data payload").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Adaptive Mode with Environmental Monitoring
//! ```rust
//! use gibberlink_core::{RgibberLink, LaserEngine, SecurityManager};
//!
//! # async fn adaptive_example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut link = RgibberLink::new();
//!
//! // Initialize security manager with weather monitoring
//! let security_config = gibberlink_core::SecurityConfig {
//!     environmental_monitoring: true,
//!     ..Default::default()
//! };
//! let mut security_manager = SecurityManager::new(security_config);
//!
//! // System automatically selects optimal mode based on:
//! // - Hardware availability (laser/ultrasound)
//! // - Environmental conditions (weather, visibility)
//! // - Range requirements
//! // - Security constraints
//!
//! link.initiate_handshake().await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Security Features
//!
//! ### Coupled Channel Authentication
//! - **Temporal Correlation**: Both channels must arrive simultaneously (±100ms)
//! - **Cross-Signature Verification**: Ultrasound authenticates laser data and vice versa
//! - **Anti-Interception**: Requires presence in both directional beams
//! - **Quality Thresholds**: Minimum signal strength and correlation requirements
//!
//! ### Advanced Cryptography
//! - **ECDH Key Exchange**: Ephemeral keys for perfect forward secrecy
//! - **AES-GCM Encryption**: Authenticated encryption with integrity
//! - **HMAC Verification**: Message authentication codes
//! - **Anti-Replay Protection**: Timestamp and nonce-based replay prevention
//!
//! ### Environmental Security
//! - **Weather Monitoring**: Adjusts security parameters based on conditions
//! - **Range Validation**: Verifies claimed distances against measurements
//! - **Beam Isolation**: Directional transmission prevents eavesdropping
//! - **Continuous Monitoring**: Runtime channel quality validation
//!
//! ## Error Handling and Fallback
//!
//! The library provides comprehensive error handling with automatic fallback:
//!
//! ### Automatic Fallback Mechanism
//! - Monitors channel quality continuously
//! - Detects degradation in laser or ultrasound channels
//! - Automatically switches to short-range mode when long-range fails
//! - Provides recovery monitoring to restore long-range when possible
//!
//! ### Error Types
//! - `HardwareUnavailable`: Required hardware not present
//! - `SafetyViolation`: Power or alignment safety limits exceeded
//! - `AlignmentLost`: Laser beam alignment lost
//! - `TemporalCouplingFailed`: Channels arrived outside timing window
//! - `DataCorruption`: ECC unable to correct transmission errors
//! - `Timeout`: Operation exceeded time limits
//!
//! ## Android Integration
//!
//! The library includes Android-specific features:
//!
//! ### JNI Interface
//! - Native C++ bridge for Android audio/laser hardware
//! - Camera integration for QR scanning and laser alignment
//! - AudioTrack/AudioRecord for parametric ultrasound
//! - Hardware acceleration for signal processing
//!
//! ### Android Permissions
//! - `RECORD_AUDIO`: For ultrasonic transmission/reception
//! - `CAMERA`: For QR scanning and laser alignment
//! - `ACCESS_FINE_LOCATION`: For environmental context (optional)
//!
//! ### Hardware Acceleration
//! - NEON SIMD for signal processing
//! - Camera2 API for high-speed laser reception
//! - Audio DSP for parametric audio generation
//!
//! ## Performance Optimization
//!
//! ### Adaptive Algorithms
//! - **Range-Based Power Control**: Adjusts laser power based on measured distance
//! - **Modulation Selection**: Chooses optimal scheme (OOK/PWM/QR) for conditions
//! - **ECC Strength Adaptation**: Increases error correction in poor conditions
//! - **Beam Steering**: Maintains alignment with moving targets
//!
//! ### Environmental Compensation
//! - **Weather Adaptation**: Adjusts parameters for rain/fog conditions
//! - **Temperature Compensation**: Accounts for air density changes
//! - **Humidity Correction**: Compensates for ultrasonic attenuation
//! - **Visibility Monitoring**: Laser power adjustment for atmospheric conditions
//!
//! ## Testing and Benchmarks
//!
//! The library includes comprehensive testing:
//!
//! ### Unit Tests
//! - Individual engine functionality
//! - Protocol state transitions
//! - Security primitive validation
//! - Error handling verification
//!
//! ### Integration Tests
//! - End-to-end handshake validation
//! - Coupled channel authentication
//! - Fallback behavior testing
//! - Cross-platform compatibility
//!
//! ### Performance Benchmarks
//! - Handshake latency measurements
//! - Throughput testing under various conditions
//! - Power consumption analysis
//! - Error rate characterization
//!
//! ### Security Tests
//! - Interception resistance validation
//! - Replay attack prevention
//! - Channel isolation testing
//! - Cryptographic primitive verification
//!
//! ## Future Extensions
//!
//! ### Planned Features
//! - **Multi-Device Coordination**: Mesh networking with multiple laser beams
//! - **Quantum-Resistant Crypto**: Post-quantum cryptographic primitives
//! - **Satellite Integration**: Long-range backup via satellite links
//! - **AI-Optimized ECC**: Machine learning enhanced error correction
//! - **Hardware Security Modules**: TPM integration for key storage
//!
//! ### Research Areas
//! - **Beam Forming Arrays**: Phased array ultrasound for extended range
//! - **Free-Space Optics**: Advanced laser communication techniques
//! - **Cognitive Radio**: Dynamic spectrum allocation for ultrasound
//! - **Environmental Adaptation**: AI-driven parameter optimization
//!

extern crate serde;

pub mod crypto;
pub mod audio;
pub mod ultrasonic_beam;
pub mod visual;
pub mod laser;
pub mod range_detector;
pub mod optical_ecc;
pub mod protocol;
pub mod channel_validator;
pub mod security;
pub mod fallback;
pub mod performance_monitor;
pub mod mission;
pub mod weather;
pub mod audit;
pub mod hierarchical;

#[cfg(feature = "python")]
pub mod python_bindings;

#[cfg(feature = "wasm")]
pub mod wasm;

pub use crypto::{CryptoEngine, CryptoError};
pub use audio::{AudioEngine, AudioError};
pub use ultrasonic_beam::{UltrasonicBeamEngine, UltrasonicBeamError, BeamConfig, BeamSignal, BeamReception};
pub use visual::{VisualEngine, VisualError, VisualPayload};
pub use laser::{LaserEngine, LaserError, LaserConfig, ReceptionConfig, AlignmentStatus, LaserType, ModulationScheme};
pub use range_detector::{RangeDetector, RangeDetectorError, RangingConfig, RangeMeasurement, RangeDetectorCategory, RangeEnvironmentalConditions};
pub use optical_ecc::{OpticalECC, OpticalECCError, OpticalQualityMetrics, AdaptiveECCConfig, AtmosphericCondition, RangeCategory};
pub use protocol::{ProtocolEngine, ProtocolError, ProtocolState, ChannelQuality};
pub use channel_validator::{ChannelValidator, ValidationError, ValidationPhase, ChannelData, ChannelType, ValidationConfig, ValidationMetrics};
pub use security::{SecurityManager, SecurityError, SecurityConfig, SecurityLevel, PermissionType, PermissionGrant, PermissionScope, PeerIdentity, TrustLevel, EnvironmentalConditions, WeatherCondition, TimeOfDay, CommandExecution};
pub use fallback::{FallbackManager, FallbackError, FallbackConfig, FallbackMode, FallbackStatus, ChannelFailure, ChannelHealth, SessionSnapshot};
pub use performance_monitor::{PerformanceMonitor, PerformanceError, PerformanceMetrics, PerformanceConfig, PerformancePreset, BenchmarkResult, EnvironmentalFactors};
pub use audit::{AuditSystem, AuditEntry, SecurityAlert, AuditEventType, AuditSeverity, AuditActor, AuditOperation, create_audit_entry};
pub use hierarchical::{HierarchicalProtocolEngine, MilitaryRank, CommandType, HierarchicalMessage, HierarchicalState, HierarchyPresence};

use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};

/// Message structure for inter-device communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub sender_fingerprint: [u8; 32],
    pub content: Vec<u8>,
    pub message_type: MessageType,
    pub timestamp: std::time::SystemTime,
    pub priority: MessagePriority,
    pub ttl_seconds: u32,
}

/// Supported message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Text(String),
    AuthorizationRequest { requested_permissions: Vec<String> },
    AuthorizationResponse { granted: bool, reason: Option<String> },
    StatusUpdate { status: String, details: String },
    Command { command: String, parameters: std::collections::HashMap<String, String> },
    Notification { title: String, body: String },
}

/// Message priority levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessagePriority {
    Low,
    Normal,
    High,
    Critical,
}

/// API Response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub success: bool,
    pub message_id: Option<String>,
    pub error: Option<String>,
    pub data: Option<serde_json::Value>,
}

/// Messaging API error types
#[derive(Debug, thiserror::Error)]
pub enum MessagingError {
    #[error("Message too large (max 64KB)")]
    MessageTooLarge,
    #[error("Invalid message format")]
    InvalidFormat,
    #[error("Permission denied")]
    PermissionDenied,
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    #[error("Connection not established")]
    ConnectionNotEstablished,
    #[error("Message expired")]
    MessageExpired,
}

/// Main RgibberLink session manager
#[derive(Clone)]
pub struct RgibberLink {
    protocol: Arc<Mutex<ProtocolEngine>>,
    message_queue: Arc<Mutex<Vec<Message>>>,
    #[allow(dead_code)]
    pending_responses: Arc<Mutex<std::collections::HashMap<String, tokio::sync::oneshot::Sender<ApiResponse>>>>,
    last_activity: Arc<Mutex<std::time::Instant>>,
    performance_monitor: Arc<Mutex<Option<PerformanceMonitor>>>,
}

impl RgibberLink {
    /// Create a new RgibberLink session
    pub fn new() -> Self {
        Self {
            protocol: Arc::new(Mutex::new(ProtocolEngine::new())),
            message_queue: Arc::new(Mutex::new(Vec::new())),
            pending_responses: Arc::new(Mutex::new(std::collections::HashMap::new())),
            last_activity: Arc::new(Mutex::new(std::time::Instant::now())),
            performance_monitor: Arc::new(Mutex::new(None)),
        }
    }

    /// Initiate the handshake as the sender
    pub async fn initiate_handshake(&mut self) -> Result<(), ProtocolError> {
        self.protocol.lock().await.initiate_handshake().await
    }

    /// Receive nonce and generate QR code as the receiver
    pub async fn receive_nonce(&self, nonce: &[u8]) -> Result<String, ProtocolError> {
        self.protocol.lock().await.receive_nonce(nonce).await
    }

    /// Process scanned QR payload
    pub async fn process_qr_payload(&mut self, qr_data: &[u8]) -> Result<(), ProtocolError> {
        self.protocol.lock().await.process_qr_payload(qr_data).await
    }

    /// Receive ACK from sender
    pub async fn receive_ack(&self) -> Result<(), ProtocolError> {
        self.protocol.lock().await.receive_ack().await
    }

    /// Get current protocol state
    pub async fn get_state(&self) -> ProtocolState {
        self.protocol.lock().await.get_state().await
    }

    /// Encrypt a message using the established session key
    pub async fn encrypt_message(&self, data: &[u8]) -> Result<Vec<u8>, ProtocolError> {
        self.protocol.lock().await.encrypt_message(data).await
    }

    /// Decrypt a message using the established session key
    pub async fn decrypt_message(&self, encrypted_data: &[u8]) -> Result<Vec<u8>, ProtocolError> {
        self.protocol.lock().await.decrypt_message(encrypted_data).await
    }

    /// Get the shared secret (for debugging/testing only)
    pub async fn get_shared_secret(&self) -> Option<[u8; 32]> {
        self.protocol.lock().await.get_shared_secret().copied()
    }

    /// Send a text message to the connected peer
    pub async fn send_text_message(&self, content: &str) -> Result<String, MessagingError> {
        self.check_connection().await?;

        let message = self.create_message(
            MessageType::Text(content.to_string()),
            MessagePriority::Normal,
            300, // 5 minute TTL
        );

        self.send_message_internal(message).await
    }

    /// Request authorization from the peer for specific permissions
    pub async fn request_authorization(&self, permissions: Vec<String>) -> Result<String, MessagingError> {
        self.check_connection().await?;

        let message = self.create_message(
            MessageType::AuthorizationRequest { requested_permissions: permissions },
            MessagePriority::High,
            60, // 1 minute TTL
        );

        self.send_message_internal(message).await
    }

    /// Respond to an authorization request
    pub async fn respond_to_authorization(&self, _request_id: &str, granted: bool, reason: Option<String>) -> Result<String, MessagingError> {
        self.check_connection().await?;

        let message = self.create_message(
            MessageType::AuthorizationResponse { granted, reason },
            MessagePriority::High,
            60,
        );

        // Link this response to the original request
        // (This would normally use a correlation ID system)

        self.send_message_internal(message).await
    }

    /// Send a status update
    pub async fn send_status_update(&self, status: &str, details: &str) -> Result<String, MessagingError> {
        self.check_connection().await?;

        let message = self.create_message(
            MessageType::StatusUpdate {
                status: status.to_string(),
                details: details.to_string()
            },
            MessagePriority::Normal,
            3600, // 1 hour TTL
        );

        self.send_message_internal(message).await
    }

    /// Send a command to the peer
    pub async fn send_command(&self, command: &str, parameters: std::collections::HashMap<String, String>) -> Result<String, MessagingError> {
        self.check_connection().await?;

        let message = self.create_message(
            MessageType::Command {
                command: command.to_string(),
                parameters
            },
            MessagePriority::High,
            300,
        );

        self.send_message_internal(message).await
    }

    /// Send a notification to the peer
    pub async fn send_notification(&self, title: &str, body: &str) -> Result<String, MessagingError> {
        self.check_connection().await?;

        let message = self.create_message(
            MessageType::Notification {
                title: title.to_string(),
                body: body.to_string()
            },
            MessagePriority::High,
            3600,
        );

        self.send_message_internal(message).await
    }

    /// Get pending messages for the application to process
    pub async fn get_pending_messages(&self) -> Vec<Message> {
        let mut queue = self.message_queue.lock().await;
        let messages = queue.clone();
        queue.clear(); // Clear processed messages
        messages
    }

    /// Check if there are pending messages
    pub async fn has_pending_messages(&self) -> bool {
        !self.message_queue.lock().await.is_empty()
    }

    /// Get recent activity timestamp
    pub async fn get_last_activity(&self) -> std::time::Instant {
        *self.last_activity.lock().await
    }

    /// Initialize performance monitoring with communication engines
    pub async fn initialize_performance_monitor(
        &self,
        laser_engine: Option<Arc<Mutex<LaserEngine>>>,
        ultrasonic_engine: Option<Arc<Mutex<UltrasonicBeamEngine>>>,
        range_detector: Option<Arc<Mutex<RangeDetector>>>,
    ) -> Result<(), PerformanceError> {
        let protocol_engine = Some(self.protocol.clone());
        let monitor = PerformanceMonitor::new(1000) // 1000 history entries
            .with_engines(laser_engine, ultrasonic_engine, range_detector, protocol_engine);

        *self.performance_monitor.lock().await = Some(monitor);
        Ok(())
    }

    /// Start performance monitoring
    pub async fn start_performance_monitoring(&self) -> Result<(), PerformanceError> {
        if let Some(monitor) = self.performance_monitor.lock().await.as_ref() {
            monitor.start_monitoring().await
        } else {
            Err(PerformanceError::InvalidMetrics)
        }
    }

    /// Stop performance monitoring
    pub async fn stop_performance_monitoring(&self) {
        if let Some(monitor) = self.performance_monitor.lock().await.as_ref() {
            monitor.stop_monitoring().await;
        }
    }

    /// Get current performance metrics
    pub async fn get_performance_metrics(&self) -> Option<PerformanceMetrics> {
        if let Some(monitor) = self.performance_monitor.lock().await.as_ref() {
            monitor.get_current_metrics().await
        } else {
            None
        }
    }

    /// Run performance benchmark suite
    pub async fn run_performance_benchmarks(&self, duration_secs: u64) -> Result<Vec<BenchmarkResult>, PerformanceError> {
        if let Some(monitor) = self.performance_monitor.lock().await.as_ref() {
            monitor.run_benchmark_suite(duration_secs).await
        } else {
            Err(PerformanceError::InvalidMetrics)
        }
    }

    /// Apply performance optimization preset
    pub async fn apply_performance_preset(&self, preset: PerformancePreset) -> Result<(), PerformanceError> {
        if let Some(monitor) = self.performance_monitor.lock().await.as_ref() {
            monitor.apply_preset(preset).await
        } else {
            Err(PerformanceError::InvalidMetrics)
        }
    }

    /// Get performance recommendations
    pub async fn get_performance_recommendations(&self) -> Vec<String> {
        if let Some(monitor) = self.performance_monitor.lock().await.as_ref() {
            monitor.get_recommendations().await
        } else {
            Vec::new()
        }
    }

    /// Process incoming encrypted message data
    pub async fn process_incoming_message(&self, encrypted_data: &[u8]) -> Result<(), MessagingError> {
        let decrypted = self.decrypt_message(encrypted_data).await
            .map_err(|_| MessagingError::InvalidFormat)?;

        let message: Message = serde_json::from_slice(&decrypted)
            .map_err(|_| MessagingError::InvalidFormat)?;

        // Update activity timestamp
        *self.last_activity.lock().await = std::time::Instant::now();

        // Handle special message types
        match &message.message_type {
            MessageType::AuthorizationRequest { .. } => {
                // This would trigger the authorization UI flow
            }
            MessageType::AuthorizationResponse { granted: false, reason } => {
                // Handle rejected authorization - could trigger notification
                self.handle_rejected_authorization(&message, reason.clone()).await?;
            }
            _ => {}
        }

        // Add to message queue for application processing
        self.message_queue.lock().await.push(message);

        Ok(())
    }

    /// Handle rejected authorization attempts with notifications
    async fn handle_rejected_authorization(&self, _message: &Message, reason: Option<String>) -> Result<(), MessagingError> {
        // Log the rejection for audit purposes
        // In a full implementation, this would trigger system notifications
        // and potentially escalate security measures

        let reason_text = reason.unwrap_or("No reason provided".to_string());

        // Create a notification message about the rejection
        let notification = self.create_message(
            MessageType::Notification {
                title: "Authorization Rejected".to_string(),
                body: format!("Peer device rejected authorization request. Reason: {}", reason_text)
            },
            MessagePriority::Critical,
            86400, // Keep for 24 hours
        );

        // This notification would be processed by the application UI
        self.message_queue.lock().await.push(notification);

        Ok(())
    }

    /// Check if we have an established connection
    async fn check_connection(&self) -> Result<(), MessagingError> {
        let state = self.get_state().await;
        match state {
            ProtocolState::Connected | ProtocolState::SecureChannelEstablished | ProtocolState::LongRangeSecureChannel => Ok(()),
            _ => Err(MessagingError::ConnectionNotEstablished),
        }
    }

    /// Create a new message with proper metadata
    fn create_message(&self, message_type: MessageType, priority: MessagePriority, ttl_seconds: u32) -> Message {
        let message_id = format!("msg_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis());

        Message {
            id: message_id,
            sender_fingerprint: [0u8; 32], // Would be set from device fingerprint
            content: Vec::new(), // Content is stored in message_type
            message_type,
            timestamp: std::time::SystemTime::now(),
            priority,
            ttl_seconds,
        }
    }

    /// Send message internally (encrypt and queue for transmission)
    async fn send_message_internal(&self, message: Message) -> Result<String, MessagingError> {
        // Check message size (64KB limit)
        let message_size = serde_json::to_vec(&message)
            .map_err(|_| MessagingError::InvalidFormat)?
            .len();

        if message_size > 65536 { // 64KB
            return Err(MessagingError::MessageTooLarge);
        }

        // Encrypt the message
        let message_bytes = serde_json::to_vec(&message)
            .map_err(|_| MessagingError::InvalidFormat)?;

        let _encrypted = self.encrypt_message(&message_bytes).await
            .map_err(|_| MessagingError::ConnectionNotEstablished)?;

        // In a full implementation, this would queue the message for transmission
        // via the appropriate channel (IR laser or ultrasound)
        // For now, we just update the activity timestamp

        *self.last_activity.lock().await = std::time::Instant::now();

        Ok(message.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_session_creation() {
        let link = RgibberLink::new();
        assert!(matches!(link.get_state().await, ProtocolState::Idle));
    }

    #[tokio::test]
    async fn test_handshake_initiation() {
        let mut _link = RgibberLink::new();

        // This would normally require audio hardware
        // For now, just test state transitions
        assert!(_link.initiate_handshake().await.is_ok());
        // State should be WaitingForQr after initiation
        // (audio sending is mocked in AudioEngine)
// FFI bindings for Android JNI
use std::ffi::{c_char, c_int, c_void};

#[no_mangle]
pub extern "C" fn gibberlink_create() -> *mut c_void {
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn gibberlink_destroy(_ptr: *mut c_void) {
}

#[no_mangle]
pub extern "C" fn gibberlink_initiate_handshake(_ptr: *mut c_void) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn gibberlink_get_state(_ptr: *mut c_void) -> c_int {
    0
}

#[no_mangle]
pub extern "C" fn gibberlink_receive_nonce(_ptr: *mut c_void, _nonce: *const u8, _nonce_len: usize) -> *const c_char {
    std::ptr::null()
}

#[no_mangle]
pub extern "C" fn gibberlink_process_qr_payload(_ptr: *mut c_void, _qr_data: *const u8, _qr_len: usize) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn gibberlink_receive_ack(_ptr: *mut c_void) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn gibberlink_encrypt_message(_ptr: *mut c_void, _data: *const u8, _data_len: usize, _out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn gibberlink_decrypt_message(_ptr: *mut c_void, _encrypted_data: *const u8, _encrypted_len: usize, _out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn gibberlink_send_audio_data(_ptr: *mut c_void, _data: *const u8, _data_len: usize) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn gibberlink_receive_audio_data(_ptr: *mut c_void, _out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn gibberlink_is_receiving(_ptr: *mut c_void) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn gibberlink_generate_qr_code(_ptr: *mut c_void, _payload: *const u8, _payload_len: usize) -> *const c_char {
    std::ptr::null()
}

#[no_mangle]
pub extern "C" fn gibberlink_decode_qr_code(_ptr: *mut c_void, _qr_data: *const u8, _qr_len: usize, _out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn ultrasonic_beam_engine_create() -> *mut c_void {
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn ultrasonic_beam_engine_destroy(_ptr: *mut c_void) {
}

#[no_mangle]
pub extern "C" fn ultrasonic_beam_engine_initialize(_ptr: *mut c_void) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn ultrasonic_beam_engine_generate_parametric_audio(_ptr: *mut c_void, _data: *const u8, _data_len: usize, _out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn ultrasonic_beam_engine_transmit_sync_pulse(_ptr: *mut c_void, _pattern: *const u8, _pattern_len: usize) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn ultrasonic_beam_engine_transmit_auth_signal(_ptr: *mut c_void, _challenge: *const u8, _challenge_len: usize, _signature: *const u8, _signature_len: usize) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn ultrasonic_beam_engine_detect_presence(_ptr: *mut c_void) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn ultrasonic_beam_engine_transmit_control_data(_ptr: *mut c_void, _data: *const u8, _data_len: usize, _priority: u8) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn ultrasonic_beam_engine_receive_beam_signals(_ptr: *mut c_void, _out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn ultrasonic_beam_engine_get_config(_ptr: *mut c_void, _out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn ultrasonic_beam_engine_update_config(_ptr: *mut c_void, _config_data: *const u8, _config_len: usize) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn ultrasonic_beam_engine_get_channel_diagnostics(_ptr: *mut c_void, _out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn ultrasonic_beam_engine_shutdown(_ptr: *mut c_void) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn laser_engine_create(_config_data: *const u8, _config_len: usize, _rx_config_data: *const u8, _rx_config_len: usize) -> *mut c_void {
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn laser_engine_destroy(_ptr: *mut c_void) {
}

#[no_mangle]
pub extern "C" fn laser_engine_initialize(_ptr: *mut c_void) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn laser_engine_shutdown(_ptr: *mut c_void) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn laser_engine_transmit_data(_ptr: *mut c_void, _data: *const u8, _data_len: usize) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn laser_engine_receive_data(_ptr: *mut c_void, _timeout_ms: u64, _out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn laser_engine_set_intensity(_ptr: *mut c_void, _intensity: f32) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn laser_engine_get_alignment_status(_ptr: *mut c_void, _out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn laser_engine_set_alignment_target(_ptr: *mut c_void, _x: f32, _y: f32) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn laser_engine_auto_align(_ptr: *mut c_void, _max_attempts: u32) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn laser_engine_get_channel_diagnostics(_ptr: *mut c_void, _out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn laser_engine_enable_adaptive_mode(_ptr: *mut c_void) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn laser_engine_disable_adaptive_mode(_ptr: *mut c_void) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn laser_engine_update_power_profile(_ptr: *mut c_void, _profile_data: *const u8, _profile_len: usize) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn laser_engine_get_current_power_profile(_ptr: *mut c_void, _out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn laser_engine_emergency_shutdown(_ptr: *mut c_void) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn laser_engine_get_safety_stats(_ptr: *mut c_void, _out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn laser_engine_reset_energy_monitoring(_ptr: *mut c_void) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn range_detector_create() -> *mut c_void {
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn range_detector_destroy(_ptr: *mut c_void) {
}

#[no_mangle]
pub extern "C" fn range_detector_initialize(_ptr: *mut c_void) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn range_detector_is_active(_ptr: *mut c_void) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn range_detector_measure_distance(_ptr: *mut c_void, _out_distance: *mut f32, _out_strength: *mut f32, _out_quality: *mut f32) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn range_detector_measure_distance_averaged(_ptr: *mut c_void, _samples: usize, _out_distance: *mut f32, _out_strength: *mut f32, _out_quality: *mut f32) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn range_detector_measure_distance_fast(_ptr: *mut c_void, _out_distance: *mut f32, _out_strength: *mut f32, _out_quality: *mut f32) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn range_detector_update_environmental_conditions(_ptr: *mut c_void, _temperature: f32, _humidity: f32, _pressure: f32, _wind_speed: f32, _visibility: f32) {
}

#[no_mangle]
pub extern "C" fn range_detector_get_environmental_conditions(_ptr: *mut c_void, _out_temperature: *mut f32, _out_humidity: *mut f32, _out_pressure: *mut f32, _out_wind_speed: *mut f32, _out_visibility: *mut f32) {
}

#[no_mangle]
pub extern "C" fn range_detector_get_current_range_category(_ptr: *mut c_void) -> c_int {
    -1
}

#[no_mangle]
pub extern "C" fn range_detector_get_measurement_history_size(_ptr: *mut c_void) -> usize {
    0
}

#[no_mangle]
pub extern "C" fn range_detector_get_measurement_history(_ptr: *mut c_void, _index: usize, _out_distance: *mut f32, _out_strength: *mut f32, _out_quality: *mut f32, _out_timestamp: *mut u64) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn range_detector_shutdown(_ptr: *mut c_void) -> bool {
    false
}

#[no_mangle]
pub extern "C" fn detect_hardware_capabilities(_out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn check_ultrasonic_hardware_available() -> bool {
    false
}

#[no_mangle]
pub extern "C" fn check_laser_hardware_available() -> bool {
    false
}

#[no_mangle]
pub extern "C" fn check_photodiode_hardware_available() -> bool {
    false
}

#[no_mangle]
pub extern "C" fn check_camera_hardware_available() -> bool {
    false
}

#[no_mangle]
pub extern "C" fn gibberlink_free_data(_data: *mut u8) {
}
    }
}
