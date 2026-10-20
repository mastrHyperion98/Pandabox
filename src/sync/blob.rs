//! Vault blob codec (docs/cloud-sync DESIGN D1, SCHEMA.md).
//!
//! A blob is a single ChaCha20-Poly1305 AEAD ciphertext over the
//! canonical vault JSON: `nonce(12) || ciphertext+tag`. The file has
//! no unencrypted prefix — magic and version live inside the
//! ciphertext. Sealing uses a fresh random nonce on every call.

use crate::encrypt::cryptography::CryptEngine;

/// Crypto failures (auth failure on a wrong key, malformed blob,
/// wrong key length). Display carries no sensitive detail.
pub type BlobError = chacha20poly1305::Error;

/// Seal `plaintext` (canonical vault JSON) with a 256-bit key.
pub fn encode_vault_blob(plaintext: &[u8], key: &[u8]) -> Result<Vec<u8>, BlobError> {
    CryptEngine::seal(plaintext, key)
}

/// Open a blob produced by `encode_vault_blob`.
/// Wrong key or tampered data ⇒ error (Poly1305 tag authenticates both).
pub fn decode_vault_blob(blob: &[u8], key: &[u8]) -> Result<Vec<u8>, BlobError> {
    CryptEngine::open(blob, key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngCore;

    fn random_key() -> Vec<u8> {
        let mut key = vec![0u8; 32];
        rand::rng().fill_bytes(&mut key);
        key
    }

    #[test]
    fn round_trip() {
        let key = random_key();
        let plaintext = b"\"magic\":\"PBXS\" fake vault json";
        let blob = encode_vault_blob(plaintext, &key).expect("encode");
        assert!(blob.len() >= 12 + 16, "nonce + tag at minimum");
        let back = decode_vault_blob(&blob, &key).expect("decode");
        assert_eq!(back.as_slice(), plaintext);
    }

    #[test]
    fn wrong_key_fails_auth() {
        let key = random_key();
        let other = random_key();
        let blob = encode_vault_blob(b"data", &key).expect("encode");
        assert!(
            decode_vault_blob(&blob, &other).is_err(),
            "Poly1305 tag must reject a wrong key, never return garbage"
        );
    }

    #[test]
    fn fresh_nonce_per_encode() {
        let key = random_key();
        let a = encode_vault_blob(b"same", &key).expect("encode a");
        let b = encode_vault_blob(b"same", &key).expect("encode b");
        assert_ne!(a, b, "nonces must never be reused with the same key");
        assert_eq!(
            decode_vault_blob(&a, &key).expect("decode a"),
            decode_vault_blob(&b, &key).expect("decode b")
        );
    }

    #[test]
    fn tampered_blob_fails() {
        let key = random_key();
        let mut blob = encode_vault_blob(b"data", &key).expect("encode");
        let i = 20usize.min(blob.len() - 1);
        blob[i] ^= 0x01;
        assert!(decode_vault_blob(&blob, &key).is_err());
    }

    #[test]
    fn rejects_bad_key_length_and_short_blob() {
        assert!(encode_vault_blob(b"data", &[0u8; 31]).is_err());
        assert!(decode_vault_blob(&[0u8; 8], &[0u8; 32]).is_err());
    }
}
