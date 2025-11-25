//! Post-Quantum Cryptographic primitives for RealGibber
//!
//! Implements Kyber-768 KEM and Dilithium3 digital signatures
//! for quantum-resistant key exchange and authentication.

use std::sync::Arc;
use zeroize::{Zeroize, ZeroizeOnDrop};
use crate::crypto::{CryptoError, EphemeralKeySession};
use std::time::{Instant, Duration};

#[cfg(feature = "post-quantum")]
use pqcrypto::{
    kem::kyber768::*,
    sign::dilithium3::*,
};

#[cfg(feature = "post-quantum")]
/// Kyber-768 Key Encapsulation Mechanism
pub struct KyberKEM;

#[cfg(feature = "post-quantum")]
impl KyberKEM {
    /// Generate a new Kyber-768 keypair
    pub fn generate_keypair() -> Result<KyberKeypair, CryptoError> {
        // Generate keypair using pqcrypto
        let (pk, sk) = keypair();
        Ok(KyberKeypair {
            public_key: pk,
            secret_key: sk,
            created_at: Instant::now(),
        })
    }

    /// Encapsulate a shared secret using recipient's public key
    pub fn encapsulate(pk: &KyberPublicKey) -> Result<KyberCiphertextData, CryptoError> {
        let (ciphertext, shared_secret) = encapsulate(pk);
        Ok(KyberCiphertextData {
            ciphertext,
            shared_secret,
        })
    }

    /// Decapsulate a shared secret using recipient's secret key
    pub fn decapsulate(sk: &KyberSecretKey, ct: &KyberCiphertext) -> Result<KyberSharedSecret, CryptoError> {
        let shared_secret = decapsulate(ct, sk);
        Ok(shared_secret)
    }
}

#[cfg(feature = "post-quantum")]
/// Dilithium3 Digital Signature
pub struct DilithiumSign;

#[cfg(feature = "post-quantum")]
impl DilithiumSign {
    /// Generate a new Dilithium3 keypair
    pub fn generate_keypair() -> Result<DilithiumKeypair, CryptoError> {
        let (pk, sk) = keypair();
        Ok(DilithiumKeypair {
            public_key: pk,
            secret_key: sk,
            created_at: Instant::now(),
        })
    }

    /// Sign a message
    pub fn sign(sk: &DilithiumSecretKey, message: &[u8]) -> Result<DilithiumSignature, CryptoError> {
        let signature = sign(message, sk);
        Ok(signature)
    }

    /// Verify a signature
    pub fn verify(pk: &DilithiumPublicKey, message: &[u8], signature: &DilithiumSignature) -> Result<bool, CryptoError> {
        Ok(verify(signature, message, pk))
    }
}

#[cfg(feature = "post-quantum")]
/// PQ Keypair structures with secure zeroization
#[derive(Clone)]
pub struct KyberKeypair {
    pub public_key: KyberPublicKey,
    pub secret_key: KyberSecretKey,
    pub created_at: Instant,
}

#[cfg(feature = "post-quantum")]
impl Zeroize for KyberKeypair {
    fn zeroize(&mut self) {
        // KyberSecretKey implements Zeroize
        self.secret_key.zeroize();
    }
}

#[cfg(feature = "post-quantum")]
impl ZeroizeOnDrop for KyberKeypair {}

#[cfg(feature = "post-quantum")]
#[derive(Clone)]
pub struct DilithiumKeypair {
    pub public_key: DilithiumPublicKey,
    pub secret_key: DilithiumSecretKey,
    pub created_at: Instant,
}

#[cfg(feature = "post-quantum")]
impl Zeroize for DilithiumKeypair {
    fn zeroize(&mut self) {
        // DilithiumSecretKey implements Zeroize
        self.secret_key.zeroize();
    }
}

#[cfg(feature = "post-quantum")]
impl ZeroizeOnDrop for DilithiumKeypair {}

#[cfg(feature = "post-quantum")]
#[derive(Clone)]
pub struct KyberCiphertextData {
    pub ciphertext: KyberCiphertext,
    pub shared_secret: KyberSharedSecret,
}

#[cfg(feature = "post-quantum")]
impl Zeroize for KyberCiphertextData {
    fn zeroize(&mut self) {
        self.ciphertext.zeroize();
        self.shared_secret.zeroize();
    }
}

#[cfg(feature = "post-quantum")]
impl ZeroizeOnDrop for KyberCiphertextData {}

// Type aliases for PQ primitives
#[cfg(feature = "post-quantum")]
pub type KyberPublicKey = pqcrypto::kem::kyber768::PublicKey;
#[cfg(feature = "post-quantum")]
pub type KyberSecretKey = pqcrypto::kem::kyber768::SecretKey;
#[cfg(feature = "post-quantum")]
pub type KyberCiphertext = pqcrypto::kem::kyber768::Ciphertext;
#[cfg(feature = "post-quantum")]
pub type KyberSharedSecret = pqcrypto::kem::kyber768::SharedSecret;

#[cfg(feature = "post-quantum")]
pub type DilithiumPublicKey = pqcrypto::sign::dilithium3::PublicKey;
#[cfg(feature = "post-quantum")]
pub type DilithiumSecretKey = pqcrypto::sign::dilithium3::SecretKey;
#[cfg(feature = "post-quantum")]
pub type DilithiumSignature = pqcrypto::sign::dilithium3::Signature;

/// Post-Quantum cryptographic engine
#[cfg(feature = "post-quantum")]
pub struct PostQuantumEngine {
    kyber_keypair: KyberKeypair,
    dilithium_keypair: DilithiumKeypair,
}

#[cfg(feature = "post-quantum")]
impl PostQuantumEngine {
    /// Create a new PQ engine with fresh keypairs
    pub fn new() -> Result<Self, CryptoError> {
        let kyber_keypair = KyberKEM::generate_keypair()?;
        let dilithium_keypair = DilithiumSign::generate_keypair()?;

        Ok(Self {
            kyber_keypair,
            dilithium_keypair,
        })
    }

    /// Get Kyber public key
    pub fn kyber_public_key(&self) -> &KyberPublicKey {
        &self.kyber_keypair.public_key
    }

    /// Get Dilithium public key
    pub fn dilithium_public_key(&self) -> &DilithiumPublicKey {
        &self.dilithium_keypair.public_key
    }

    /// Perform PQ key encapsulation
    pub fn encapsulate_secret(&self, peer_pk: &KyberPublicKey) -> Result<KyberCiphertextData, CryptoError> {
        KyberKEM::encapsulate(peer_pk)
    }

    /// Perform PQ key decapsulation
    pub fn decapsulate_secret(&self, ciphertext: &KyberCiphertext) -> Result<KyberSharedSecret, CryptoError> {
        KyberKEM::decapsulate(&self.kyber_keypair.secret_key, ciphertext)
    }

    /// Sign data with Dilithium3
    pub fn sign_data(&self, data: &[u8]) -> Result<DilithiumSignature, CryptoError> {
        DilithiumSign::sign(&self.dilithium_keypair.secret_key, data)
    }

    /// Verify Dilithium3 signature
    pub fn verify_signature(&self, data: &[u8], signature: &DilithiumSignature, public_key: &DilithiumPublicKey) -> Result<bool, CryptoError> {
        DilithiumSign::verify(public_key, data, signature)
    }
}

#[cfg(not(feature = "post-quantum"))]
/// Stub implementation when PQ features are not enabled
pub struct PostQuantumEngine;

#[cfg(not(feature = "post-quantum")]
impl PostQuantumEngine {
    pub fn new() -> Result<Self, CryptoError> {
        Err(CryptoError::GenericError("Post-quantum cryptography not enabled".to_string()))
    }
}

#[cfg(test)]
#[cfg(feature = "post-quantum")]
mod tests {
    use super::*;

    #[test]
    fn test_kyber_kem() {
        let alice = KyberKEM::generate_keypair().unwrap();
        let bob = KyberKEM::generate_keypair().unwrap();

        // Alice encapsulates for Bob
        let alice_ct = KyberKEM::encapsulate(&bob.public_key).unwrap();

        // Bob decapsulates
        let bob_ss = KyberKEM::decapsulate(&bob.secret_key, &alice_ct.ciphertext).unwrap();

        // Verify shared secrets match
        assert_eq!(alice_ct.shared_secret.as_bytes(), bob_ss.as_bytes());
    }

    #[test]
    fn test_dilithium_sign() {
        let alice = DilithiumSign::generate_keypair().unwrap();
        let message = b"Hello, quantum-resistant world!";

        // Sign message
        let signature = DilithiumSign::sign(&alice.secret_key, message).unwrap();

        // Verify signature
        let valid = DilithiumSign::verify(&alice.public_key, message, &signature).unwrap();
        assert!(valid);

        // Verify with wrong message fails
        let invalid = DilithiumSign::verify(&alice.public_key, b"Wrong message", &signature).unwrap();
        assert!(!invalid);
    }

    #[test]
    fn test_pq_engine() {
        let alice = PostQuantumEngine::new().unwrap();
        let bob = PostQuantumEngine::new().unwrap();

        // Test key exchange
        let alice_ct = alice.encapsulate_secret(bob.kyber_public_key()).unwrap();
        let bob_ss = bob.decapsulate_secret(&alice_ct.ciphertext).unwrap();

        assert_eq!(alice_ct.shared_secret.as_bytes(), bob_ss.as_bytes());

        // Test signatures
        let message = b"Authenticated quantum message";
        let signature = alice.sign_data(message).unwrap();
        let verified = bob.verify_signature(message, &signature, alice.dilithium_public_key()).unwrap();
        assert!(verified);
    }
}