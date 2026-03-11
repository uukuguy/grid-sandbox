//! Cryptographic signing for Byzantine consensus.
//!
//! Uses ED25519 for message signing and verification. Each agent in the
//! consensus protocol holds a [`ConsensusKeypair`] that it uses to sign
//! votes and proposals. Other agents verify the signature before accepting
//! a message, preventing impersonation and tampering.

use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use rand::Rng;
use serde::{Deserialize, Serialize};

/// A keypair for signing consensus messages.
///
/// Wraps an ED25519 signing key and its corresponding verifying (public) key,
/// bound to a specific `agent_id`.
pub struct ConsensusKeypair {
    signing_key: SigningKey,
    /// The public (verifying) half of this keypair.
    pub verifying_key: VerifyingKey,
    /// The agent that owns this keypair.
    pub agent_id: String,
}

/// A signed message wrapper that includes the signature and signer's public key.
///
/// This is the on-the-wire representation: the original payload bytes, the 64-byte
/// ED25519 signature, the 32-byte public key, and the human-readable agent ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedMessage {
    /// The serialized message bytes.
    pub payload: Vec<u8>,
    /// ED25519 signature (64 bytes).
    pub signature: Vec<u8>,
    /// ED25519 public key (32 bytes).
    pub signer_public_key: Vec<u8>,
    /// Human-readable identifier of the signing agent.
    pub agent_id: String,
}

/// Result of verifying a [`SignedMessage`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyResult {
    /// Signature is valid for the given payload and public key.
    Valid,
    /// The signature does not match the payload / public key pair.
    InvalidSignature,
    /// The embedded public key bytes could not be deserialized.
    KeyMismatch,
    /// Payload deserialization failed (with reason).
    DeserializationError(String),
}

impl ConsensusKeypair {
    /// Generates a new random ED25519 keypair for the given agent.
    pub fn generate(agent_id: String) -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        Self {
            signing_key,
            verifying_key,
            agent_id,
        }
    }

    /// Signs an arbitrary byte slice and returns a [`SignedMessage`].
    pub fn sign(&self, message: &[u8]) -> SignedMessage {
        let signature = self.signing_key.sign(message);
        SignedMessage {
            payload: message.to_vec(),
            signature: signature.to_bytes().to_vec(),
            signer_public_key: self.verifying_key.to_bytes().to_vec(),
            agent_id: self.agent_id.clone(),
        }
    }

    /// Returns the raw public key bytes (32 bytes).
    pub fn public_key_bytes(&self) -> Vec<u8> {
        self.verifying_key.to_bytes().to_vec()
    }

    /// Returns the raw private key bytes (32 bytes).
    pub fn private_key_bytes(&self) -> Vec<u8> {
        self.signing_key.to_bytes().to_vec()
    }

    /// Encrypts the private key using AES-GCM for secure storage.
    /// Returns (encrypted_data, nonce).
    pub fn encrypt_private_key(&self, key: &[u8; 32]) -> Result<(Vec<u8>, Vec<u8>), String> {
        let cipher = Aes256Gcm::new(key.into());
        let nonce_bytes: [u8; 12] = rand::thread_rng().gen();
        let nonce = Nonce::from_slice(&nonce_bytes);
        let private_bytes = self.signing_key.to_bytes();
        let encrypted = cipher
            .encrypt(nonce, private_bytes.as_ref())
            .map_err(|e| format!("encryption failed: {}", e))?;
        Ok((encrypted, nonce_bytes.to_vec()))
    }

    /// Restores a keypair from encrypted private key data.
    pub fn decrypt_and_restore(
        agent_id: &str,
        public_key_bytes: &[u8],
        encrypted_private: &[u8],
        nonce: &[u8],
        key: &[u8; 32],
    ) -> Result<Self, String> {
        let cipher = Aes256Gcm::new(key.into());
        let nonce = Nonce::from_slice(nonce);
        let private_bytes = cipher
            .decrypt(nonce, encrypted_private)
            .map_err(|e| format!("decryption failed: {}", e))?;
        let private_array: [u8; 32] = private_bytes
            .try_into()
            .map_err(|_| "invalid private key length".to_string())?;
        let signing_key = SigningKey::from_bytes(&private_array);
        let verifying_key = signing_key.verifying_key();

        // Verify the public key matches
        let expected: [u8; 32] = public_key_bytes
            .try_into()
            .map_err(|_| "invalid public key length".to_string())?;
        if verifying_key.to_bytes() != expected {
            return Err("public key mismatch after decryption".to_string());
        }

        Ok(Self {
            signing_key,
            verifying_key,
            agent_id: agent_id.to_string(),
        })
    }
}

/// Verifies a [`SignedMessage`] using the embedded public key and signature.
///
/// Returns [`VerifyResult::Valid`] when the signature matches the payload,
/// [`VerifyResult::KeyMismatch`] when the public key bytes are malformed,
/// and [`VerifyResult::InvalidSignature`] otherwise.
pub fn verify_signature(signed: &SignedMessage) -> VerifyResult {
    // Reconstruct verifying key from bytes.
    let key_bytes: [u8; 32] = match signed.signer_public_key.as_slice().try_into() {
        Ok(b) => b,
        Err(_) => return VerifyResult::KeyMismatch,
    };
    let verifying_key = match VerifyingKey::from_bytes(&key_bytes) {
        Ok(k) => k,
        Err(_) => return VerifyResult::KeyMismatch,
    };

    // Reconstruct signature from bytes.
    let sig_bytes: [u8; 64] = match signed.signature.as_slice().try_into() {
        Ok(b) => b,
        Err(_) => return VerifyResult::InvalidSignature,
    };
    let signature = Signature::from_bytes(&sig_bytes);

    match verifying_key.verify(&signed.payload, &signature) {
        Ok(()) => VerifyResult::Valid,
        Err(_) => VerifyResult::InvalidSignature,
    }
}

/// Convenience: serializes consensus-vote data into a deterministic byte
/// representation, then signs it with the given keypair.
pub fn sign_consensus_vote(
    keypair: &ConsensusKeypair,
    proposal_id: &str,
    phase: &str,
    approve: bool,
) -> SignedMessage {
    let payload = format!(
        "vote:{}:{}:{}:{}",
        proposal_id, phase, approve, keypair.agent_id
    );
    keypair.sign(payload.as_bytes())
}

/// Convenience: verifies a signed consensus vote and checks that the embedded
/// payload matches the expected proposal ID and phase.
///
/// Returns [`VerifyResult::Valid`] only when *both* the cryptographic signature
/// is valid *and* the payload contains the expected proposal/phase fields.
pub fn verify_consensus_vote(
    signed: &SignedMessage,
    expected_proposal_id: &str,
    expected_phase: &str,
) -> VerifyResult {
    // First verify the raw cryptographic signature.
    let sig_result = verify_signature(signed);
    if sig_result != VerifyResult::Valid {
        return sig_result;
    }

    // Then check semantic content.
    let payload_str = match std::str::from_utf8(&signed.payload) {
        Ok(s) => s,
        Err(e) => return VerifyResult::DeserializationError(e.to_string()),
    };

    // Expected format: "vote:{proposal_id}:{phase}:{approve}:{agent_id}"
    let parts: Vec<&str> = payload_str.splitn(5, ':').collect();
    if parts.len() < 4 {
        return VerifyResult::DeserializationError("invalid vote payload format".into());
    }

    if parts[0] != "vote" {
        return VerifyResult::DeserializationError("payload does not start with 'vote'".into());
    }
    if parts[1] != expected_proposal_id {
        return VerifyResult::DeserializationError(format!(
            "proposal_id mismatch: expected '{}', got '{}'",
            expected_proposal_id, parts[1]
        ));
    }
    if parts[2] != expected_phase {
        return VerifyResult::DeserializationError(format!(
            "phase mismatch: expected '{}', got '{}'",
            expected_phase, parts[2]
        ));
    }

    VerifyResult::Valid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_produces_unique_keys() {
        let kp1 = ConsensusKeypair::generate("a1".into());
        let kp2 = ConsensusKeypair::generate("a2".into());
        assert_ne!(kp1.public_key_bytes(), kp2.public_key_bytes());
    }

    #[test]
    fn sign_and_verify_roundtrip() {
        let kp = ConsensusKeypair::generate("agent-1".into());
        let signed = kp.sign(b"hello world");
        assert_eq!(verify_signature(&signed), VerifyResult::Valid);
    }

    #[test]
    fn tampered_payload_detected() {
        let kp = ConsensusKeypair::generate("agent-1".into());
        let mut signed = kp.sign(b"original");
        signed.payload = b"tampered".to_vec();
        assert_eq!(verify_signature(&signed), VerifyResult::InvalidSignature);
    }

    #[test]
    fn wrong_key_fails_verification() {
        let kp1 = ConsensusKeypair::generate("a1".into());
        let kp2 = ConsensusKeypair::generate("a2".into());
        let mut signed = kp1.sign(b"data");
        // Replace public key with kp2's key
        signed.signer_public_key = kp2.public_key_bytes();
        assert_eq!(verify_signature(&signed), VerifyResult::InvalidSignature);
    }

    #[test]
    fn consensus_vote_sign_verify_roundtrip() {
        let kp = ConsensusKeypair::generate("voter".into());
        let signed = sign_consensus_vote(&kp, "prop-1", "Prepare", true);
        assert_eq!(
            verify_consensus_vote(&signed, "prop-1", "Prepare"),
            VerifyResult::Valid
        );
    }

    #[test]
    fn consensus_vote_wrong_proposal_fails() {
        let kp = ConsensusKeypair::generate("voter".into());
        let signed = sign_consensus_vote(&kp, "prop-1", "Prepare", true);
        let result = verify_consensus_vote(&signed, "prop-WRONG", "Prepare");
        match result {
            VerifyResult::DeserializationError(msg) => {
                assert!(msg.contains("proposal_id mismatch"));
            }
            other => panic!("Expected DeserializationError, got {:?}", other),
        }
    }
}
