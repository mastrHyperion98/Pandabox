//! Vault blob codec (docs/cloud-sync DESIGN D1, SCHEMA.md).
//!
//! Blob layout (v2): `nonce(12) || ciphertext+tag || salt(32)`.
//! The AEAD part is a single ChaCha20-Poly1305 ciphertext over the
//! canonical vault JSON. The 12-byte nonce is a public parameter
//! (standard for AEAD), and the appended 32-byte salt is the vault's
//! master-record salt — public by design — so a fresh machine can
//! derive a candidate key from the master password and authenticate
//! the blob without any local state.
//!
//! Sealing uses a fresh random nonce on every call.

use crate::encrypt::cryptography::CryptEngine;
use zeroize::Zeroizing;

/// Crypto failures (auth failure on a wrong key, malformed blob,
/// wrong key length). Display carries no sensitive detail.
pub type BlobError = chacha20poly1305::Error;

pub const SALT_LENGTH: usize = 32;

/// Seal `plaintext` (canonical vault JSON) with a 256-bit key and
/// append the vault salt (public parameter for password-based restore).
pub fn encode_vault_blob(
    plaintext: &[u8],
    key: &[u8],
    salt: &[u8],
) -> Result<Vec<u8>, BlobError> {
    if salt.len() != SALT_LENGTH {
        return Err(chacha20poly1305::Error);
    }
    let sealed = CryptEngine::seal(plaintext, key)?;
    let mut out = sealed;
    out.extend_from_slice(salt);
    Ok(out)
}

/// Open a blob produced by `encode_vault_blob` with the key.
/// Wrong key or tampered data ⇒ error (Poly1305 tag authenticates both).
pub fn decode_vault_blob(blob: &[u8], key: &[u8]) -> Result<Vec<u8>, BlobError> {
    if blob.len() < 12 + 16 + SALT_LENGTH {
        return Err(chacha20poly1305::Error);
    }
    CryptEngine::open(&blob[..blob.len() - SALT_LENGTH], key)
}

/// Open a blob on a machine with no local vault state: the master
/// password plus the blob's public salt derive a candidate key.
/// Returns `(canonical vault JSON, engine)`; the JSON is a `Zeroizing`
/// buffer, wiped on drop. Callers needing the derived key use
/// `engine.derived_key()` (borrow) rather than a second copy.
pub fn open_blob_with_password(
    blob: &[u8],
    password: &str,
) -> Result<(Zeroizing<Vec<u8>>, CryptEngine), BlobError> {
    if blob.len() < 12 + 16 + SALT_LENGTH {
        return Err(chacha20poly1305::Error);
    }
    let salt = &blob[blob.len() - SALT_LENGTH..];
    let engine = CryptEngine::new(password, salt).map_err(|_| chacha20poly1305::Error)?;
    let json = Zeroizing::new(decode_vault_blob(blob, engine.derived_key())?);
    Ok((json, engine))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::RngCore;
    use zeroize::Zeroizing;

    fn random_key() -> Zeroizing<Vec<u8>> {
        let mut key = vec![0u8; 32];
        rand::rng().fill_bytes(&mut key);
        Zeroizing::new(key)
    }

    #[test]
    fn round_trip() {
        let key = random_key();
        let salt = [0x5A; 32];
        let plaintext = b"\"magic\":\"PBXS\" fake vault json";
        let blob = encode_vault_blob(plaintext, &key, &salt).expect("encode");
        assert_eq!(blob.len(), 12 + plaintext.len() + 16 + 32, "nonce + json+tag + salt");
        assert_eq!(&blob[blob.len() - 32..], &salt, "salt is the public suffix");
        let back = decode_vault_blob(&blob, &key).expect("decode");
        assert_eq!(back.as_slice(), plaintext);
    }

    #[test]
    fn wrong_key_fails_auth() {
        let key = random_key();
        let other = random_key();
        let blob = encode_vault_blob(b"data", &key, &[0u8; 32]).expect("encode");
        assert!(
            decode_vault_blob(&blob, &other).is_err(),
            "Poly1305 tag must reject a wrong key, never return garbage"
        );
    }

    #[test]
    fn fresh_nonce_per_encode() {
        let key = random_key();
        let a = encode_vault_blob(b"same", &key, &[0u8; 32]).expect("encode a");
        let b = encode_vault_blob(b"same", &key, &[0u8; 32]).expect("encode b");
        assert_ne!(a, b, "nonces must never be reused with the same key");
        assert_eq!(
            decode_vault_blob(&a, &key).expect("decode a"),
            decode_vault_blob(&b, &key).expect("decode b")
        );
    }

    #[test]
    fn tampered_blob_fails() {
        let key = random_key();
        let mut blob = encode_vault_blob(b"data", &key, &[0u8; 32]).expect("encode");
        let i = 20usize.min(blob.len() - 1);
        blob[i] ^= 0x01;
        assert!(decode_vault_blob(&blob, &key).is_err());
    }

    #[test]
    fn rejects_bad_lengths() {
        assert!(encode_vault_blob(b"data", &[0u8; 31], &[0u8; 32]).is_err());
        assert!(encode_vault_blob(b"data", &[0u8; 32], &[0u8; 31]).is_err());
        assert!(decode_vault_blob(&[0u8; 20], &[0u8; 32]).is_err());
    }
}
