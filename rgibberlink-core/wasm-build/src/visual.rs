use qrcode::QrCode;
use reed_solomon_erasure::galois_8::ReedSolomon;
use serde::{Deserialize, Serialize};
use serde_cbor;
use crc32fast;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, thiserror::Error)]
pub enum VisualError {
    #[error("QR code generation failed")]
    QrCodeError,
    #[error("Reed-Solomon encoding failed")]
    ReedSolomonError,
    #[error("CBOR serialization failed")]
    CborError,
    #[error("Data too large for QR code")]
    DataTooLarge,
    #[error("Invalid compensation state")]
    InvalidCompensationState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualPayload {
    pub session_id: [u8; 16],
    pub public_key: Vec<u8>,
    pub nonce: [u8; 16],
    pub signature: Vec<u8>,
}

/// Compensation protocol states for noisy environments
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CompensationState {
    /// Sender: initiate, display S for 1s
    SenderInitiate = b'S' as isize,
    /// Receiver: ready, display L up to 2s
    ReceiverListen = b'L' as isize,
    /// Receiver: acknowledge, display R for 1s then 2s
    ReceiverAck = b'R' as isize,
    /// Either: close session, display F for 1s
    Finish = b'F' as isize,
    /// Error: retry, display E for 1s then revert
    ErrorRecover = b'E' as isize,
}

/// Visual compensation frame for noisy environment mode
#[derive(Serialize, Deserialize)]
pub struct CompensationFrame {
    pub state: CompensationState,
    pub session_id: [u8; 16],
    pub sequence_id: u32,
    pub timestamp: u64,
    pub payload: Option<Vec<u8>>, // MAC confirmation + ultrasonic profile
    pub crc: u32,
}

impl CompensationState {
    pub fn as_char(&self) -> char {
        match self {
            CompensationState::SenderInitiate => 'S',
            CompensationState::ReceiverListen => 'L',
            CompensationState::ReceiverAck => 'R',
            CompensationState::Finish => 'F',
            CompensationState::ErrorRecover => 'E',
        }
    }

    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'S' => Some(CompensationState::SenderInitiate),
            'L' => Some(CompensationState::ReceiverListen),
            'R' => Some(CompensationState::ReceiverAck),
            'F' => Some(CompensationState::Finish),
            'E' => Some(CompensationState::ErrorRecover),
            _ => None,
        }
    }
}

impl CompensationFrame {
    pub fn new(state: CompensationState, session_id: [u8; 16], sequence_id: u32, payload: Option<Vec<u8>>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let mut frame = Self {
            state,
            session_id,
            sequence_id,
            timestamp,
            payload,
            crc: 0,
        };

        // Calculate CRC over all fields except crc itself
        let cbor_data = serde_cbor::to_vec(&frame).unwrap_or_default();
        frame.crc = crc32fast::hash(&cbor_data[..cbor_data.len().saturating_sub(4)]);

        frame
    }
}

#[derive(Debug)]
pub struct VisualEngine {
    rs: ReedSolomon,
}

impl VisualEngine {
    pub fn new() -> Self {
        // Reed-Solomon with 8 data shards and 4 parity shards for 12 total
        let rs = ReedSolomon::new(8, 4).expect("Failed to create Reed-Solomon codec");
        Self { rs }
    }

    pub fn encode_payload(&self, payload: &VisualPayload) -> Result<String, VisualError> {
        // Serialize to CBOR
        let cbor_data = serde_cbor::to_vec(payload).map_err(|_| VisualError::CborError)?;

        // Compress data (simple length-prefixed for prototype)
        let mut compressed = (cbor_data.len() as u16).to_le_bytes().to_vec();
        compressed.extend(cbor_data);

        // Split into shards
        let shard_size = (compressed.len() + 7) / 8; // Ceiling division
        let mut shards: Vec<Vec<u8>> = Vec::with_capacity(12);

        for i in 0..8 {
            let start = i * shard_size;
            let end = std::cmp::min(start + shard_size, compressed.len());
            let mut shard = compressed[start..end].to_vec();
            // Pad shard to shard_size
            shard.resize(shard_size, 0);
            shards.push(shard);
        }

        // Add parity shards
        shards.resize(12, vec![0; shard_size]);
        self.rs.encode(&mut shards).map_err(|_| VisualError::ReedSolomonError)?;

        // Flatten into one vector for QR
        let mut encoded_data = Vec::new();
        for shard in &shards {
            encoded_data.extend(shard);
        }

        // Generate QR code
        if encoded_data.len() > 2953 { // Max data for QR version 40
            return Err(VisualError::DataTooLarge);
        }

        let code = QrCode::new(&encoded_data).map_err(|_| VisualError::QrCodeError)?;
        let svg = code.render::<qrcode::render::svg::Color>().build();

        Ok(svg)
    }

    pub fn decode_payload(&self, qr_data: &[u8]) -> Result<VisualPayload, VisualError> {
        // Parse QR data (simplified - assume raw bytes)
        let total_size = qr_data.len();
        let shard_size = (total_size + 11) / 12; // Assuming 12 shards

        let mut shards: Vec<Option<Vec<u8>>> = Vec::with_capacity(12);

        for i in 0..12 {
            let start = i * shard_size;
            let end = std::cmp::min(start + shard_size, total_size);
            shards.push(Some(qr_data[start..end].to_vec()));
        }

        // Reconstruct data
        self.rs.reconstruct(&mut shards).map_err(|_| VisualError::ReedSolomonError)?;

        // Collect data shards
        let mut reconstructed = Vec::new();
        for shard in shards.into_iter().take(8).flatten() {
            reconstructed.extend(shard);
        }

        // Decompress (remove length prefix)
        if reconstructed.len() < 2 {
            return Err(VisualError::CborError);
        }
        let data_len = u16::from_le_bytes([reconstructed[0], reconstructed[1]]) as usize;
        if reconstructed.len() < 2 + data_len {
            return Err(VisualError::CborError);
        }
        let cbor_data = &reconstructed[2..2 + data_len];

        // Deserialize from CBOR
        let payload: VisualPayload = serde_cbor::from_slice(cbor_data).map_err(|_| VisualError::CborError)?;

        Ok(payload)
    }

    /// Encode compensation frame with enhanced layout for noisy environments
    pub fn encode_compensation_frame(&self, frame: &CompensationFrame) -> Result<String, VisualError> {
        // Serialize frame
        let cbor_data = serde_cbor::to_vec(frame).map_err(|_| VisualError::CborError)?;

        // Add state code as first byte for fast detection
        let mut data_with_state = vec![frame.state.as_char() as u8];
        data_with_state.extend(&cbor_data);

        // Enhanced Reed-Solomon for noisy environments (more parity)
        let rs_compensation = ReedSolomon::new(12, 6).map_err(|_| VisualError::ReedSolomonError)?;

        // Split into shards
        let shard_size = (data_with_state.len() + 11) / 12;
        let mut shards: Vec<Vec<u8>> = Vec::with_capacity(18);

        for i in 0..12 {
            let start = i * shard_size;
            let end = std::cmp::min(start + shard_size, data_with_state.len());
            let mut shard = data_with_state[start..end].to_vec();
            shard.resize(shard_size, 0);
            shards.push(shard);
        }

        // Add parity
        shards.resize(18, vec![0; shard_size]);
        rs_compensation.encode(&mut shards).map_err(|_| VisualError::ReedSolomonError)?;

        // Arrange layout: state in top-left, data in center, parity at bottom
        let mut encoded_data = Vec::new();

        // Top-left: state code repeated and CRC checksum
        encoded_data.push(frame.state.as_char() as u8);
        encoded_data.extend_from_slice(&frame.crc.to_le_bytes());

        // Central: interleaved session_id and sequence
        encoded_data.extend_from_slice(&frame.session_id);
        encoded_data.extend_from_slice(&frame.sequence_id.to_le_bytes());

        // Rest: ECC parity blocks
        for shard in &shards[12..] {
            encoded_data.extend(shard);
        }

        // Generate QR with enhanced contrast for noisy environments
        let code = QrCode::new(&encoded_data).map_err(|_| VisualError::QrCodeError)?;

        // Use high contrast colors and add border
        let svg = code
            .render::<qrcode::render::svg::Color>()
            .dark_color(qrcode::render::svg::Color("#000000"))
            .light_color(qrcode::render::svg::Color("#FFFFFF"))
            .build();

        Ok(svg)
    }

    /// Decode compensation frame with motion blur tolerance
    pub fn decode_compensation_frame(&self, qr_data: &[u8]) -> Result<CompensationFrame, VisualError> {
        if qr_data.len() < 1 {
            return Err(VisualError::InvalidCompensationState);
        }

        // Extract state from first byte
        let state_char = qr_data[0] as char;
        let state = CompensationState::from_char(state_char)
            .ok_or(VisualError::InvalidCompensationState)?;

        // Extract CRC and verify
        if qr_data.len() < 5 {
            return Err(VisualError::CborError);
        }
        let expected_crc = u32::from_le_bytes([qr_data[1], qr_data[2], qr_data[3], qr_data[4]]);

        // Try to reconstruct CBOR data from the rest
        let cbor_start = 5;
        if qr_data.len() < cbor_start + 4 {
            return Err(VisualError::CborError);
        }

        // Enhanced reconstruction for noisy environments
        let rs_compensation = ReedSolomon::new(12, 6).map_err(|_| VisualError::ReedSolomonError)?;

        let shard_size = ((qr_data.len() - cbor_start) + 11) / 12;
        let mut shards: Vec<Option<Vec<u8>>> = Vec::with_capacity(18);

        // First 12 data shards from session_id/sequence area
        let center_data = &qr_data[cbor_start..std::cmp::min(qr_data.len(), cbor_start + 12 * shard_size)];
        for i in 0..12 {
            let start = i * shard_size;
            let end = std::cmp::min(start + shard_size, center_data.len());
            shards.push(Some(center_data[start..end].to_vec()));
        }

        // Attempt reconstruction
        if rs_compensation.reconstruct_data(&mut shards).is_ok() {
            let mut reconstructed = Vec::new();
            for shard in shards.into_iter().take(12).flatten() {
                reconstructed.extend(shard);
            }

            // Deserialize frame
            let frame: CompensationFrame = serde_cbor::from_slice(&reconstructed)
                .map_err(|_| VisualError::CborError)?;

            // Verify CRC
            if crc32fast::hash(&serde_cbor::to_vec(&frame).unwrap_or_default()[..reconstructed.len().saturating_sub(4)]) == expected_crc {
                return Ok(frame);
            }
        }

        Err(VisualError::ReedSolomonError)
    }
}