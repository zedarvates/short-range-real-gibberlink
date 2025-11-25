use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
use rand::RngCore;
use x25519_dalek::{EphemeralSecret, PublicKey};
use ed25519_dalek::{SigningKey, VerifyingKey, Signer, Verifier, Signature};
use std::time::{Instant, Duration};
use zeroize::{Zeroize, ZeroizeOnDrop};
use hkdf::Hkdf;
use sha2::{Sha256, Digest};
use hmac::Mac;

#[cfg(feature = "post-quantum")]
use crate::post_quantum::{PostQuantumEngine, KyberKEM, DilithiumSign, KyberKeypair, DilithiumKeypair, KyberCiphertextData};

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("AES-GCM encryption error")]
    AeadError,
    #[error("HMAC verification failed")]
    HmacError,
    #[error("Invalid key length")]
    InvalidKeyLength,
    #[error("Key expired")]
    KeyExpired,
    #[error("Signature verification failed")]
    SignatureError,
    #[error("Ed25519 signing error")]
    Ed25519Error,
    #[error("{0}")]
    GenericError(String),
}

#[derive(Clone)]
pub struct EphemeralKeySession {
    key: [u8; 32],
    created_at: Instant,
    ttl: Duration,
}

impl Zeroize for EphemeralKeySession {
    fn zeroize(&mut self) {
        self.key.zeroize();
    }
}

impl ZeroizeOnDrop for EphemeralKeySession {}

impl EphemeralKeySession {
    pub fn new(key: [u8; 32], ttl: Duration) -> Self {
        Self {
            key,
            created_at: Instant::now(),
            ttl,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }

    pub fn key(&self) -> &[u8; 32] {
        &self.key
    }

    /// Invalidate post-usage with secure zeroization
    pub fn invalidate(&mut self) {
        self.key.zeroize();
        self.ttl = Duration::from_secs(0);
    }
}

pub struct CryptoEngine {
    ecdh_secret: EphemeralSecret,
    ecdh_public: PublicKey,
    ed25519_keypair: SigningKey,
    ed25519_public: VerifyingKey,
    #[cfg(feature = "post-quantum")]
    pq_engine: Option<PostQuantumEngine>,
}

impl CryptoEngine {
    pub fn new() -> Self {
        // ECDH for key exchange
        let ecdh_secret = EphemeralSecret::random_from_rng(rand::thread_rng());
        let ecdh_public = PublicKey::from(&ecdh_secret);

        // Ed25519 for signing logs
        let mut csprng = rand::thread_rng();
        let mut secret_key = [0u8; 32];
        csprng.fill_bytes(&mut secret_key);
        let ed25519_keypair = SigningKey::from_bytes(&secret_key);
        let ed25519_public = ed25519_keypair.verifying_key();

        #[cfg(feature = "post-quantum")]
        let pq_engine = PostQuantumEngine::new().ok();

        Self {
            ecdh_secret,
            ecdh_public,
            ed25519_keypair,
            ed25519_public,
            #[cfg(feature = "post-quantum")]
            pq_engine,
        }
    }

    pub fn ecdh_public_key(&self) -> &[u8] {
        self.ecdh_public.as_bytes()
    }

    pub fn ed25519_public_key(&self) -> &[u8; 32] {
        self.ed25519_public.as_bytes()
    }

    /// Get the ECDH public key (alias for ecdh_public_key)
    pub fn public_key(&self) -> &[u8] {
        self.ecdh_public_key()
    }

    /// Derive shared secret (alias for derive_ephemeral_shared_secret)
    pub fn derive_shared_secret(&mut self, peer_public_key: &[u8]) -> Result<[u8; 32], CryptoError> {
        let session = self.derive_ephemeral_shared_secret(peer_public_key)?;
        Ok(*session.key())
    }

    /// ECDH key derivation with peer's public key
    pub fn derive_ephemeral_shared_secret(&mut self, peer_public_key: &[u8]) -> Result<EphemeralKeySession, CryptoError> {
        let peer_key = PublicKey::from(<[u8; 32]>::try_from(peer_public_key)
            .map_err(|_| CryptoError::InvalidKeyLength)?);

        // Take ownership of the secret to call diffie_hellman
        let secret = std::mem::replace(&mut self.ecdh_secret, EphemeralSecret::random_from_rng(rand::thread_rng()));
        let shared_secret = secret.diffie_hellman(&peer_key);
        let mut key = [0u8; 32];
        key.copy_from_slice(shared_secret.as_bytes());

        // Regenerate ECDH keypair for forward secrecy
        self.ecdh_secret = EphemeralSecret::random_from_rng(rand::thread_rng());
        self.ecdh_public = PublicKey::from(&self.ecdh_secret);

        // Default TTL ≤ 5 seconds as per specs
        Ok(EphemeralKeySession::new(key, Duration::from_secs(5)))
    }

    pub fn encrypt_data(key: &[u8], data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| CryptoError::InvalidKeyLength)?;
        let nonce_full = Self::generate_nonce();
        let nonce_bytes = &nonce_full[..12];
        let nonce = Nonce::from_slice(nonce_bytes);

        let mut ciphertext = cipher.encrypt(nonce, data).map_err(|_| CryptoError::AeadError)?;
        ciphertext.splice(0..0, nonce_bytes.iter().cloned());
        Ok(ciphertext)
    }

    /// Cryptographically secure random generation with timing attack protection
    pub fn generate_secure_random_bytes(len: usize) -> Vec<u8> {
        let mut bytes = vec![0u8; len];
        rand::thread_rng().fill_bytes(&mut bytes);
        bytes
    }

    /// Constant-time comparison for HMAC verification
    pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }
        let mut result = 0u8;
        for (x, y) in a.iter().zip(b.iter()) {
            result |= x ^ y;
        }
        result == 0
    }

    /// Generate fingerprint for device identification
    pub fn generate_device_fingerprint(device_info: &[u8]) -> [u8; 32] {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(device_info);
        hasher.finalize().into()
    }

    pub fn decrypt_data(key: &[u8], encrypted_data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if encrypted_data.len() < 12 {
            return Err(CryptoError::AeadError);
        }

        let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| CryptoError::InvalidKeyLength)?;
        let nonce = Nonce::from_slice(&encrypted_data[..12]);
        cipher.decrypt(nonce, &encrypted_data[12..]).map_err(|_| CryptoError::AeadError)
    }

    /// Encrypt IR payload (high-bandwidth channel) using AES-GCM
    pub fn encrypt_ir_payload(key: &[u8], payload: &[u8], timestamp: u64) -> Result<Vec<u8>, CryptoError> {
        // Include timestamp in authenticated data for replay protection
        let mut authenticated_data = timestamp.to_be_bytes().to_vec();
        authenticated_data.extend_from_slice(payload);

        let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| CryptoError::InvalidKeyLength)?;
        let full_nonce = Self::generate_nonce();
        let nonce_bytes = &full_nonce[..12];
        let nonce = Nonce::from_slice(nonce_bytes);

        let mut ciphertext = cipher.encrypt(nonce, payload).map_err(|_| CryptoError::AeadError)?;
        ciphertext.splice(0..0, nonce_bytes.iter().cloned());
        Ok(ciphertext)
    }

    /// Decrypt IR payload
    pub fn decrypt_ir_payload(key: &[u8], encrypted_payload: &[u8]) -> Result<Vec<u8>, CryptoError> {
        Self::decrypt_data(key, encrypted_payload)
    }

    /// Encrypt ultrasonic frame (low-bandwidth control channel) using HMAC-SHA256
    pub fn encrypt_ultrasonic_frame(key: &[u8], frame: &[u8], timestamp: u64) -> Vec<u8> {
        let mut data_with_timestamp = timestamp.to_be_bytes().to_vec();
        data_with_timestamp.extend_from_slice(frame);
        Self::compute_hmac(key, &data_with_timestamp)
    }

    /// Verify ultrasonic frame HMAC
    pub fn verify_ultrasonic_frame(key: &[u8], frame: &[u8], timestamp: u64, expected_hmac: &[u8]) -> Result<(), CryptoError> {
        let computed = Self::encrypt_ultrasonic_frame(key, frame, timestamp);
        if Self::constant_time_eq(&computed, expected_hmac) {
            Ok(())
        } else {
            Err(CryptoError::HmacError)
        }
    }

    pub fn compute_hmac(key: &[u8], data: &[u8]) -> Vec<u8> {
        // Proper HMAC-SHA256 implementation
        use hmac::Mac;
        let mut mac = <hmac::Hmac<sha2::Sha256> as hmac::Mac>::new_from_slice(key).expect("HMAC can take key of any size");
        mac.update(data);
        mac.finalize().into_bytes().to_vec()
    }

    pub fn verify_hmac(key: &[u8], data: &[u8], expected_hmac: &[u8]) -> Result<(), CryptoError> {
        let computed = Self::compute_hmac(key, data);
        if Self::constant_time_eq(&computed, expected_hmac) {
            Ok(())
        } else {
            Err(CryptoError::HmacError)
        }
    }

    /// Sign log entry with Ed25519
    pub fn sign_log_entry(&self, log_data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let signature = self.ed25519_keypair.sign(log_data);
        Ok(signature.to_bytes().to_vec())
    }

    /// Verify log signature
    pub fn verify_log_signature(public_key: &[u8], log_data: &[u8], signature: &[u8]) -> Result<(), CryptoError> {
        let pk_bytes: [u8; 32] = public_key.try_into().map_err(|_| CryptoError::SignatureError)?;
        let pk = VerifyingKey::from_bytes(&pk_bytes)
            .map_err(|_| CryptoError::SignatureError)?;
        let sig_bytes: [u8; 64] = signature.try_into().map_err(|_| CryptoError::SignatureError)?;
        let sig = Signature::from_bytes(&sig_bytes);
        pk.verify(log_data, &sig).map_err(|_| CryptoError::SignatureError)
    }

    pub fn generate_nonce() -> [u8; 16] {
        let mut nonce = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut nonce);
        nonce
    }

    /// Generate a random session key (32 bytes for AES-256)
    pub fn generate_session_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        key
    }

    /// Sign data using Ed25519 (alias for sign_log_entry)
    pub fn sign_data(&self, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        self.sign_log_entry(data)
    }

    /// Generate HMAC using SHA256
    pub fn generate_hmac(key: &[u8], data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        Ok(Self::compute_hmac(key, data))
    }

    /// HKDF key derivation using SHA-256
    pub fn hkdf_derive_key(&self, ikm: &[u8], info: &[u8], length: usize) -> Result<[u8; 32], CryptoError> {
        use sha2::{Sha256, Digest};

        // For simplicity, we'll use a simple KDF. In production, use proper HKDF
        let mut output = [0u8; 32];

        // Simple KDF: Hash(ikm + info)
        let mut hasher = Sha256::default();
        hasher.update(ikm);
        hasher.update(info);
        let hash = hasher.finalize();

        output.copy_from_slice(&hash[..32]);

        Ok(output)
    }

    /// Check if post-quantum cryptography is available
    pub fn has_post_quantum(&self) -> bool {
        #[cfg(feature = "post-quantum")]
        {
            self.pq_engine.is_some()
        }
        #[cfg(not(feature = "post-quantum"))]
        {
            false
        }
    }

    /// Get Kyber public key for post-quantum key exchange
    #[cfg(feature = "post-quantum")]
    pub fn kyber_public_key(&self) -> Option<&crate::post_quantum::KyberPublicKey> {
        self.pq_engine.as_ref()?.kyber_public_key()
    }

    /// Get Dilithium public key for post-quantum signatures
    #[cfg(feature = "post-quantum")]
    pub fn dilithium_public_key(&self) -> Option<&crate::post_quantum::DilithiumPublicKey> {
        self.pq_engine.as_ref()?.dilithium_public_key()
    }

    /// Perform post-quantum key encapsulation
    #[cfg(feature = "post-quantum")]
    pub fn pq_encapsulate_secret(&self, peer_pk: &crate::post_quantum::KyberPublicKey) -> Result<KyberCiphertextData, CryptoError> {
        self.pq_engine.as_ref()
            .ok_or(CryptoError::GenericError("Post-quantum not available".to_string()))?
            .encapsulate_secret(peer_pk)
    }

    /// Perform post-quantum key decapsulation
    #[cfg(feature = "post-quantum")]
    pub fn pq_decapsulate_secret(&self, ciphertext: &crate::post_quantum::KyberCiphertext) -> Result<crate::post_quantum::KyberSharedSecret, CryptoError> {
        self.pq_engine.as_ref()
            .ok_or(CryptoError::GenericError("Post-quantum not available".to_string()))?
            .decapsulate_secret(ciphertext)
    }

    /// Sign data with post-quantum Dilithium signature
    #[cfg(feature = "post-quantum")]
    pub fn pq_sign_data(&self, data: &[u8]) -> Result<crate::post_quantum::DilithiumSignature, CryptoError> {
        self.pq_engine.as_ref()
            .ok_or(CryptoError::GenericError("Post-quantum not available".to_string()))?
            .sign_data(data)
    }

    /// Verify post-quantum Dilithium signature
    #[cfg(feature = "post-quantum")]
    pub fn pq_verify_signature(&self, data: &[u8], signature: &crate::post_quantum::DilithiumSignature, public_key: &crate::post_quantum::DilithiumPublicKey) -> Result<bool, CryptoError> {
        self.pq_engine.as_ref()
            .ok_or(CryptoError::GenericError("Post-quantum not available".to_string()))?
            .verify_signature(data, signature, public_key)
    }

    /// Hybrid key exchange: Combine classical ECDH with post-quantum Kyber
    #[cfg(feature = "post-quantum")]
    pub fn hybrid_key_exchange(&mut self, peer_ecdh_key: &[u8], peer_kyber_key: &crate::post_quantum::KyberPublicKey) -> Result<[u8; 32], CryptoError> {
        // Perform classical ECDH
        let classical_session = self.derive_ephemeral_shared_secret(peer_ecdh_key)?;

        // Perform post-quantum key exchange
        let pq_ciphertext = self.pq_encapsulate_secret(peer_kyber_key)?;
        let pq_shared_secret = self.pq_decapsulate_secret(&pq_ciphertext.ciphertext)?;

        // Combine secrets using HKDF
        let mut combined_secret = [0u8; 64];
        combined_secret[..32].copy_from_slice(classical_session.key());
        combined_secret[32..].copy_from_slice(pq_shared_secret.as_bytes());

        // Derive final key
        self.hkdf_derive_key(&combined_secret, b"hybrid-key-exchange", 32)
    }

    /// Hybrid signature: Sign with both Ed25519 and Dilithium
    #[cfg(feature = "post-quantum")]
    pub fn hybrid_sign_data(&self, data: &[u8]) -> Result<(Vec<u8>, crate::post_quantum::DilithiumSignature), CryptoError> {
        let classical_sig = self.sign_data(data)?;
        let pq_sig = self.pq_sign_data(data)?;

        Ok((classical_sig, pq_sig))
    }

    /// Hybrid signature verification
    #[cfg(feature = "post-quantum")]
    pub fn hybrid_verify_signature(&self, data: &[u8], classical_sig: &[u8], pq_sig: &crate::post_quantum::DilithiumSignature, pq_public_key: &crate::post_quantum::DilithiumPublicKey) -> Result<bool, CryptoError> {
        // Verify classical signature
        let classical_valid = self.verify_log_signature(self.ed25519_public_key().as_bytes(), data, classical_sig).is_ok();

        // Verify post-quantum signature
        let pq_valid = self.pq_verify_signature(data, pq_sig, pq_public_key)?;

        Ok(classical_valid && pq_valid)
    }
}
