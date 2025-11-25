//! Multi-Modal AI Data Exchange Module
//!
//! This module provides support for AI-to-AI communication with multi-modal data,
//! including audio, video, image, and sensor data exchange capabilities.

use crate::crypto::CryptoEngine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// AI Message structure for LLM communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AI_Message {
    pub session_id: [u8; 32],
    pub sequence_number: u64,
    pub timestamp: std::time::SystemTime,
    pub sender_fingerprint: [u8; 32],
    pub message_type: AI_MessageType,
    pub payload: Vec<u8>, // Encrypted AI data
    pub signature: [u8; 64],
}

/// Multi-modal metadata for AI messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiModalMetadata {
    pub content_types: Vec<ContentType>,
    pub total_size_bytes: u64,
    pub compression_info: Option<CompressionInfo>,
    pub integrity_hashes: Vec<IntegrityHash>,
    pub security_level: SecurityLevel,
    pub range_optimized: bool,
}

/// Supported content types for multi-modal data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentType {
    Text,
    Audio {
        sample_rate: u32,
        channels: u8,
        format: AudioFormat
    },
    Video {
        width: u32,
        height: u32,
        frame_rate: f32,
        format: VideoFormat
    },
    Image {
        width: u32,
        height: u32,
        format: ImageFormat
    },
    SensorData {
        sensor_type: SensorType,
        precision: u8
    },
    BinaryData { mime_type: String },
}

/// Payload format specifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PayloadFormat {
    Raw,
    Compressed(CompressionAlgorithm),
    Chunked { total_chunks: u32, chunk_index: u32 },
    Hybrid(Vec<PayloadFormat>),
}

/// Chunk information for large data transmission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkInfo {
    pub chunk_id: u32,
    pub total_chunks: u32,
    pub chunk_size: u32,
    pub total_size: u64,
    pub chunk_hash: [u8; 32],
}

/// Compression information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionInfo {
    pub algorithm: CompressionAlgorithm,
    pub original_size: u64,
    pub compressed_size: u64,
    pub compression_ratio: f32,
}

/// Supported compression algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    None,
    Brotli,
    Zstd,
    Lz4,
    Adaptive,
}

/// Integrity hash for data verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityHash {
    pub algorithm: HashAlgorithm,
    pub hash: Vec<u8>,
    pub offset: u64,
    pub length: u64,
}

/// Supported hash algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HashAlgorithm {
    Sha256,
    Blake3,
    Crc32,
}

/// Security levels for multi-modal data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityLevel {
    Standard,
    High,
    Critical,
}

/// Audio format specifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudioFormat {
    Pcm,
    Opus,
    Aac,
    Flac,
}

/// Video format specifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VideoFormat {
    H264,
    H265,
    Vp8,
    Vp9,
}

/// Image format specifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImageFormat {
    Jpeg,
    Png,
    Webp,
    Tiff,
}

/// Sensor type specifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SensorType {
    Lidar,
    Radar,
    Thermal,
    Imu,
    Gps,
    Environmental,
}

/// AI Message types including multi-modal support
pub enum AI_MessageType {
    Prompt,
    Response,
    ContextUpdate,
    CoordinationSignal,
    ErrorReport,
    // Multi-modal message types
    AudioData,
    VideoData,
    ImageData,
    SensorData,
    MixedMedia,
    StreamChunk,
    StreamEnd,
    QualityRequest,
    QualityResponse,
    CompressionNegotiation,
}

/// Payload encoder for multi-modal data
pub struct PayloadEncoder {
    compression_engines: HashMap<CompressionAlgorithm, Box<dyn CompressionEngine>>,
    integrity_verifier: IntegrityVerifier,
}

impl PayloadEncoder {
    pub fn new() -> Self {
        let mut compression_engines = HashMap::new();
        compression_engines.insert(CompressionAlgorithm::Brotli, Box::new(BrotliEngine::new()));
        compression_engines.insert(CompressionAlgorithm::Zstd, Box::new(ZstdEngine::new()));
        compression_engines.insert(CompressionAlgorithm::Lz4, Box::new(Lz4Engine::new()));

        Self {
            compression_engines,
            integrity_verifier: IntegrityVerifier::new(),
        }
    }

    pub async fn encode_payload(
        &self,
        data: &[u8],
        content_type: &ContentType,
        range_meters: f32
    ) -> Result<EncodedPayload, EncodingError> {
        // Adaptive compression based on content type and range
        let algorithm = self.select_compression_algorithm(content_type, range_meters, data.len())?;

        let compressed_data = if let Some(engine) = self.compression_engines.get(&algorithm) {
            engine.compress(data).await?
        } else {
            data.to_vec()
        };

        // Generate integrity hashes
        let integrity_hashes = self.integrity_verifier.generate_hashes(&compressed_data).await?;

        // Create compression info
        let compression_info = if algorithm != CompressionAlgorithm::None {
            Some(CompressionInfo {
                algorithm,
                original_size: data.len() as u64,
                compressed_size: compressed_data.len() as u64,
                compression_ratio: data.len() as f32 / compressed_data.len() as f32,
            })
        } else {
            None
        };

        Ok(EncodedPayload {
            data: compressed_data,
            compression_info,
            integrity_hashes,
            format: PayloadFormat::Compressed(algorithm),
        })
    }

    fn select_compression_algorithm(
        &self,
        content_type: &ContentType,
        range_meters: f32,
        data_size: usize
    ) -> Result<CompressionAlgorithm, EncodingError> {
        match content_type {
            ContentType::Audio { .. } => {
                // Audio benefits from Brotli for general compression
                if range_meters > 100.0 {
                    Ok(CompressionAlgorithm::Lz4) // Faster for long range
                } else {
                    Ok(CompressionAlgorithm::Brotli)
                }
            }
            ContentType::Video { .. } => {
                // Video needs high compression ratios
                Ok(CompressionAlgorithm::Zstd)
            }
            ContentType::Image { .. } => {
                // Images compress well with Brotli
                Ok(CompressionAlgorithm::Brotli)
            }
            ContentType::SensorData { .. } => {
                // Sensor data often has patterns, use adaptive
                if data_size > 1024 * 1024 {
                    Ok(CompressionAlgorithm::Adaptive)
                } else {
                    Ok(CompressionAlgorithm::Lz4)
                }
            }
            _ => Ok(CompressionAlgorithm::None),
        }
    }
}

/// Chunk manager for large data transmission
pub struct ChunkManager {
    max_chunk_size: usize,
    integrity_verifier: IntegrityVerifier,
}

impl ChunkManager {
    pub fn new(max_chunk_size: usize) -> Self {
        Self {
            max_chunk_size,
            integrity_verifier: IntegrityVerifier::new(),
        }
    }

    pub async fn create_chunks(
        &self,
        data: &[u8],
        session_id: &[u8; 32],
        sequence_start: u64
    ) -> Result<Vec<MessageChunk>, ChunkingError> {
        let total_chunks = ((data.len() + self.max_chunk_size - 1) / self.max_chunk_size) as u32;
        let mut chunks = Vec::new();

        for (i, chunk_data) in data.chunks(self.max_chunk_size).enumerate() {
            let chunk_id = i as u32;
            let sequence_number = sequence_start + i as u64;

            // Generate chunk hash
            let chunk_hash = self.integrity_verifier.hash_sha256(chunk_data).await?;

            let chunk = MessageChunk {
                session_id: *session_id,
                sequence_number,
                chunk_info: ChunkInfo {
                    chunk_id,
                    total_chunks,
                    chunk_size: chunk_data.len() as u32,
                    total_size: data.len() as u64,
                    chunk_hash,
                },
                payload: chunk_data.to_vec(),
                signature: self.integrity_verifier.sign_chunk(chunk_data, session_id, sequence_number).await?,
            };

            chunks.push(chunk);
        }

        Ok(chunks)
    }

    pub async fn reassemble_chunks(&self, chunks: &[MessageChunk]) -> Result<Vec<u8>, ChunkingError> {
        if chunks.is_empty() {
            return Ok(Vec::new());
        }

        // Sort chunks by chunk_id
        let mut sorted_chunks = chunks.to_vec();
        sorted_chunks.sort_by_key(|c| c.chunk_info.chunk_id);

        // Verify chunk integrity and ordering
        for (i, chunk) in sorted_chunks.iter().enumerate() {
            if chunk.chunk_info.chunk_id != i as u32 {
                return Err(ChunkingError::InvalidChunkOrder);
            }

            // Verify chunk hash
            let computed_hash = self.integrity_verifier.hash_sha256(&chunk.payload).await?;
            if computed_hash != chunk.chunk_info.chunk_hash {
                return Err(ChunkingError::IntegrityCheckFailed);
            }
        }

        // Reassemble payload
        let mut reassembled = Vec::new();
        for chunk in sorted_chunks {
            reassembled.extend_from_slice(&chunk.payload);
        }

        Ok(reassembled)
    }
}

/// Encoded payload structure
#[derive(Debug)]
pub struct EncodedPayload {
    pub data: Vec<u8>,
    pub compression_info: Option<CompressionInfo>,
    pub integrity_hashes: Vec<IntegrityHash>,
    pub format: PayloadFormat,
}

/// Message chunk structure
#[derive(Debug, Clone)]
pub struct MessageChunk {
    pub session_id: [u8; 32],
    pub sequence_number: u64,
    pub chunk_info: ChunkInfo,
    pub payload: Vec<u8>,
    pub signature: Vec<u8>,
}

// Placeholder implementations for compression engines
trait CompressionEngine {
    fn compress(&self, data: &[u8]) -> impl std::future::Future<Output = Result<Vec<u8>, EncodingError>> + Send;
}

struct BrotliEngine;
struct ZstdEngine;
struct Lz4Engine;

impl BrotliEngine {
    fn new() -> Self { Self }
}

impl ZstdEngine {
    fn new() -> Self { Self }
}

impl Lz4Engine {
    fn new() -> Self { Self }
}

impl CompressionEngine for BrotliEngine {
    async fn compress(&self, data: &[u8]) -> Result<Vec<u8>, EncodingError> {
        // Placeholder: In real implementation, use brotli crate
        Ok(data.to_vec())
    }
}

impl CompressionEngine for ZstdEngine {
    async fn compress(&self, data: &[u8]) -> Result<Vec<u8>, EncodingError> {
        // Placeholder: In real implementation, use zstd crate
        Ok(data.to_vec())
    }
}

impl CompressionEngine for Lz4Engine {
    async fn compress(&self, data: &[u8]) -> Result<Vec<u8>, EncodingError> {
        // Placeholder: In real implementation, use lz4 crate
        Ok(data.to_vec())
    }
}

/// Integrity verifier for data validation
pub struct IntegrityVerifier {
    crypto_engine: Arc<Mutex<CryptoEngine>>,
}

impl IntegrityVerifier {
    pub fn new() -> Self {
        Self {
            crypto_engine: Arc::new(Mutex::new(CryptoEngine::new())),
        }
    }

    pub async fn generate_hashes(&self, data: &[u8]) -> Result<Vec<IntegrityHash>, EncodingError> {
        let sha256_hash = self.hash_sha256(data).await?;
        let blake3_hash = self.hash_blake3(data).await?;

        Ok(vec![
            IntegrityHash {
                algorithm: HashAlgorithm::Sha256,
                hash: sha256_hash.to_vec(),
                offset: 0,
                length: data.len() as u64,
            },
            IntegrityHash {
                algorithm: HashAlgorithm::Blake3,
                hash: blake3_hash.to_vec(),
                offset: 0,
                length: data.len() as u64,
            },
        ])
    }

    pub async fn hash_sha256(&self, data: &[u8]) -> Result<[u8; 32], EncodingError> {
        // Placeholder: In real implementation, use sha2 crate
        let mut hash = [0u8; 32];
        // Simple placeholder hash
        for (i, &byte) in data.iter().enumerate() {
            hash[i % 32] ^= byte;
        }
        Ok(hash)
    }

    pub async fn hash_blake3(&self, data: &[u8]) -> Result<[u8; 32], EncodingError> {
        // Placeholder: In real implementation, use blake3 crate
        let mut hash = [0u8; 32];
        // Simple placeholder hash
        for (i, &byte) in data.iter().enumerate() {
            hash[i % 32] ^= byte.wrapping_add(1);
        }
        Ok(hash)
    }

    pub async fn sign_chunk(&self, data: &[u8], session_id: &[u8; 32], sequence: u64) -> Result<Vec<u8>, EncodingError> {
        let mut crypto = self.crypto_engine.lock().await;
        // Placeholder: In real implementation, use proper signing
        let mut signature_data = Vec::new();
        signature_data.extend_from_slice(session_id);
        signature_data.extend_from_slice(&sequence.to_le_bytes());
        signature_data.extend_from_slice(data);

        // Simple HMAC placeholder
        let hmac = crypto.compute_hmac(&[0u8; 32], &signature_data);
        Ok(hmac)
    }
}

/// Encoding error types
#[derive(Debug, thiserror::Error)]
pub enum EncodingError {
    #[error("Compression failed: {0}")]
    CompressionError(String),
    #[error("Integrity verification failed: {0}")]
    IntegrityError(String),
    #[error("Unsupported content type")]
    UnsupportedContentType,
}

/// Chunking error types
#[derive(Debug, thiserror::Error)]
pub enum ChunkingError {
    #[error("Invalid chunk order")]
    InvalidChunkOrder,
    #[error("Integrity check failed")]
    IntegrityCheckFailed,
    #[error("Chunk size exceeded")]
    ChunkSizeExceeded,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_payload_encoder_creation() {
        let encoder = PayloadEncoder::new();
        assert!(encoder.compression_engines.len() > 0);
    }

    #[tokio::test]
    async fn test_chunk_manager_creation() {
        let manager = ChunkManager::new(1024);
        assert_eq!(manager.max_chunk_size, 1024);
    }

    #[tokio::test]
    async fn test_integrity_verifier_creation() {
        let verifier = IntegrityVerifier::new();
        // Test basic functionality
        let data = b"test data";
        let hashes = verifier.generate_hashes(data).await.unwrap();
        assert!(!hashes.is_empty());
    }
/// Federated Learning structures and implementations

/// Range-aware communication context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeContext {
    pub estimated_distance: f32, // meters
    pub signal_quality: f32,      // 0.0 to 1.0
    pub communication_mode: CommunicationMode,
}

/// Communication modes for range optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommunicationMode {
    ShortRange,
    LongRange,
    UltraLongRange,
}

/// Participant state in federated learning
#[derive(Debug, Clone)]
pub struct ParticipantState {
    pub fingerprint: [u8; 32],
    pub last_update: std::time::SystemTime,
    pub contribution_count: u32,
    pub trust_score: f32,
    pub range_context: Option<RangeContext>,
}

/// Gradient update structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradientUpdate {
    pub model_version: u64,
    pub gradients: HashMap<String, Vec<f32>>, // layer_name -> gradient vector
    pub batch_size: u32,
    pub learning_rate: f32,
}

/// Communication profile for range optimization
#[derive(Debug, Clone)]
pub struct CommunicationProfile {
    pub max_payload_size: usize,
    pub compression_level: u8,
    pub priority_boost: bool,
    pub reliability_mode: ReliabilityMode,
}

/// Reliability modes for different ranges
#[derive(Debug, Clone)]
pub enum ReliabilityMode {
    BestEffort,
    Acknowledged,
    Guaranteed,
}

/// Differential Privacy Engine
pub struct DifferentialPrivacyEngine {
    epsilon: f64,
    delta: f64,
    sensitivity: f64,
}

impl DifferentialPrivacyEngine {
    pub fn new(epsilon: f64, delta: f64, sensitivity: f64) -> Self {
        Self { epsilon, delta, sensitivity }
    }

    pub fn add_noise_to_gradients(&self, gradients: &HashMap<String, Vec<f32>>) -> Result<HashMap<String, Vec<f32>>, ProtocolError> {
        let mut noisy_gradients = HashMap::new();
        let noise_scale = (2.0 * self.sensitivity / self.epsilon).sqrt();

        for (layer_name, grads) in gradients {
            let mut noisy_grads = Vec::new();
            for &grad in grads {
                // Add Laplace noise
                let noise = self.laplace_noise(noise_scale);
                noisy_grads.push(grad + noise as f32);
            }
            noisy_gradients.insert(layer_name.clone(), noisy_grads);
        }

        Ok(noisy_gradients)
    }

    fn laplace_noise(&self, scale: f64) -> f64 {
        // Simple Laplace noise implementation (placeholder)
        // In real implementation, use proper statistical libraries
        use std::f64::consts::PI;
        let u: f64 = rand::random();
        let sign = if rand::random::<bool>() { 1.0 } else { -1.0 };
        sign * scale * (1.0 - 2.0 * u).ln()
    }
}

/// Secure Aggregation Engine
pub struct SecureAggregationEngine {
    minimum_participants: usize,
    participant_gradients: HashMap<[u8; 32], HashMap<String, Vec<f32>>>,
}

impl SecureAggregationEngine {
    pub fn new(minimum_participants: usize) -> Self {
        Self {
            minimum_participants,
            participant_gradients: HashMap::new(),
        }
    }

    pub fn add_participant(&mut self, fingerprint: [u8; 32]) {
        self.participant_gradients.insert(fingerprint, HashMap::new());
    }

    pub fn store_gradients(&mut self, participant: [u8; 32], gradients: HashMap<String, Vec<f32>>) -> Result<(), ProtocolError> {
        if let Some(participant_store) = self.participant_gradients.get_mut(&participant) {
            *participant_store = gradients;
            Ok(())
        } else {
            Err(ProtocolError::InvalidParticipant)
        }
    }

    pub async fn aggregate_gradients(&mut self) -> Result<HashMap<String, Vec<f32>>, ProtocolError> {
        if self.participant_gradients.len() < self.minimum_participants {
            return Err(ProtocolError::InsufficientParticipants);
        }

        let mut aggregated = HashMap::new();

        // Get all layer names from first participant
        if let Some((_, first_gradients)) = self.participant_gradients.iter().next() {
            for layer_name in first_gradients.keys() {
                let mut layer_gradients = Vec::new();

                // Collect gradients from all participants for this layer
                for participant_grads in self.participant_gradients.values() {
                    if let Some(grads) = participant_grads.get(layer_name) {
                        layer_gradients.push(grads.clone());
                    }
                }

                if !layer_gradients.is_empty() {
                    // Average the gradients
                    let avg_gradients = self.average_gradients(&layer_gradients);
                    aggregated.insert(layer_name.clone(), avg_gradients);
                }
            }
        }

        // Clear stored gradients after aggregation
        for participant_store in self.participant_gradients.values_mut() {
            participant_store.clear();
        }

        Ok(aggregated)
    }

    fn average_gradients(&self, gradient_sets: &[Vec<f32>]) -> Vec<f32> {
        if gradient_sets.is_empty() {
            return Vec::new();
        }

        let num_participants = gradient_sets.len();
        let gradient_size = gradient_sets[0].len();
        let mut averaged = vec![0.0; gradient_size];

        for grads in gradient_sets {
            for (i, &grad) in grads.iter().enumerate() {
                if i < gradient_size {
                    averaged[i] += grad;
                }
            }
        }

        for grad in &mut averaged {
            *grad /= num_participants as f32;
        }

        averaged
    }
}

/// Range-Aware Optimizer
pub struct RangeAwareOptimizer {
    profiles: HashMap<CommunicationMode, CommunicationProfile>,
}

impl RangeAwareOptimizer {
    pub fn new() -> Self {
        let mut profiles = HashMap::new();

        profiles.insert(CommunicationMode::ShortRange, CommunicationProfile {
            max_payload_size: 1024 * 1024, // 1MB
            compression_level: 1, // Low compression for speed
            priority_boost: false,
            reliability_mode: ReliabilityMode::BestEffort,
        });

        profiles.insert(CommunicationMode::LongRange, CommunicationProfile {
            max_payload_size: 512 * 1024, // 512KB
            compression_level: 3, // Medium compression
            priority_boost: false,
            reliability_mode: ReliabilityMode::Acknowledged,
        });

        profiles.insert(CommunicationMode::UltraLongRange, CommunicationProfile {
            max_payload_size: 128 * 1024, // 128KB
            compression_level: 5, // High compression
            priority_boost: true,
            reliability_mode: ReliabilityMode::Guaranteed,
        });

        Self { profiles }
    }

    pub fn get_profile(&self, mode: CommunicationMode) -> CommunicationProfile {
        self.profiles.get(&mode).cloned().unwrap_or_else(|| {
            // Default profile
            CommunicationProfile {
                max_payload_size: 512 * 1024,
                compression_level: 3,
                priority_boost: false,
                reliability_mode: ReliabilityMode::Acknowledged,
            }
        })
    }
}

/// Federated Learning coordinator
pub struct FederatedLearningCoordinator {
    session_key: EncryptionKey,
    participants: HashMap<[u8; 32], ParticipantState>,
    aggregation_engine: SecureAggregationEngine,
    privacy_engine: DifferentialPrivacyEngine,
    range_optimizer: RangeAwareOptimizer,
}

impl FederatedLearningCoordinator {
    pub fn new(session_key: EncryptionKey) -> Self {
        Self {
            session_key,
            participants: HashMap::new(),
            aggregation_engine: SecureAggregationEngine::new(3), // Minimum 3 participants
            privacy_engine: DifferentialPrivacyEngine::new(1.0, 1e-5, 1.0), // ε=1.0, δ=1e-5, sensitivity=1.0
            range_optimizer: RangeAwareOptimizer::new(),
        }
    }

    /// Register a new participant in the federated learning session
    pub fn register_participant(&mut self, fingerprint: [u8; 32], range_context: Option<RangeContext>) -> Result<(), ProtocolError> {
        let participant = ParticipantState {
            fingerprint,
            last_update: std::time::SystemTime::now(),
            contribution_count: 0,
            trust_score: 1.0,
            range_context,
        };
        self.participants.insert(fingerprint, participant);
        self.aggregation_engine.add_participant(fingerprint);
        Ok(())
    }

    /// Process gradient update from participant
    pub async fn process_gradient_update(
        &mut self,
        participant_id: [u8; 32],
        gradient_update: GradientUpdate,
    ) -> Result<(), ProtocolError> {
        // Verify participant is registered
        let participant = self.participants.get_mut(&participant_id)
            .ok_or(ProtocolError::InvalidParticipant)?;

        // Apply differential privacy noise
        let privatized_gradients = self.privacy_engine.add_noise_to_gradients(&gradient_update.gradients)?;

        // Store the privatized gradients for aggregation
        self.aggregation_engine.store_gradients(participant_id, privatized_gradients)?;

        // Update participant state
        participant.last_update = std::time::SystemTime::now();
        participant.contribution_count += 1;

        Ok(())
    }

    /// Perform secure aggregation of gradients
    pub async fn perform_aggregation(&mut self) -> Result<HashMap<String, Vec<f32>>, ProtocolError> {
        self.aggregation_engine.aggregate_gradients().await
    }

    /// Optimize communication based on range context
    pub fn optimize_for_range(&self, range_context: &RangeContext) -> CommunicationProfile {
        self.range_optimizer.get_profile(range_context.communication_mode)
    }
}

#[cfg(test)]
mod fl_tests {
    use super::*;

    #[tokio::test]
    async fn test_differential_privacy_engine() {
        let engine = DifferentialPrivacyEngine::new(1.0, 1e-5, 1.0);
        let mut gradients = HashMap::new();
        gradients.insert("layer1".to_string(), vec![1.0, 2.0, 3.0]);

        let noisy_gradients = engine.add_noise_to_gradients(&gradients).unwrap();
        assert_eq!(noisy_gradients.len(), 1);
        assert_eq!(noisy_gradients["layer1"].len(), 3);
    }

    #[tokio::test]
    async fn test_secure_aggregation_engine() {
        let mut engine = SecureAggregationEngine::new(2);

        let participant1 = [1u8; 32];
        let participant2 = [2u8; 32];

        engine.add_participant(participant1);
        engine.add_participant(participant2);

        // Add gradients from participant 1
        let mut grads1 = HashMap::new();
        grads1.insert("layer1".to_string(), vec![1.0, 2.0]);
        engine.store_gradients(participant1, grads1).unwrap();

        // Add gradients from participant 2
        let mut grads2 = HashMap::new();
        grads2.insert("layer1".to_string(), vec![3.0, 4.0]);
        engine.store_gradients(participant2, grads2).unwrap();

        // Aggregate gradients
        let aggregated = engine.aggregate_gradients().await.unwrap();
        assert_eq!(aggregated["layer1"], vec![2.0, 3.0]); // Average of [1,2] and [3,4]
    }

    #[test]
    fn test_range_optimizer() {
        let optimizer = RangeAwareOptimizer::new();

        let short_range_profile = optimizer.get_profile(CommunicationMode::ShortRange);
        assert_eq!(short_range_profile.max_payload_size, 1024 * 1024);

        let ultra_long_profile = optimizer.get_profile(CommunicationMode::UltraLongRange);
        assert_eq!(ultra_long_profile.max_payload_size, 128 * 1024);
        assert!(ultra_long_profile.priority_boost);
    }
}
}