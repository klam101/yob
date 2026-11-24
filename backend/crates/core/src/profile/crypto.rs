//! AES-256-GCM for the equal-employment answers. A fresh random 96-bit nonce
//! per write; the nonce is stored next to the ciphertext.

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};

use crate::config::ProfileKey;

/// Returns (nonce, ciphertext).
pub fn encrypt(key: &ProfileKey, plaintext: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let cipher = Aes256Gcm::new_from_slice(&key.0).expect("32-byte key");
    let mut nonce = [0u8; 12];
    getrandom::fill(&mut nonce).expect("system random number generator");
    let ct = cipher
        .encrypt(&Nonce::from(nonce), plaintext)
        .expect("AES-GCM encryption of an in-memory buffer");
    (nonce.to_vec(), ct)
}

/// None when the key is wrong or the data was altered.
pub fn decrypt(key: &ProfileKey, nonce: &[u8], ciphertext: &[u8]) -> Option<Vec<u8>> {
    let nonce: [u8; 12] = nonce.try_into().ok()?;
    let cipher = Aes256Gcm::new_from_slice(&key.0).ok()?;
    cipher.decrypt(&Nonce::from(nonce), ciphertext).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_tamper() {
        let key = ProfileKey([7; 32]);
        let (n, ct) = encrypt(&key, b"{\"gender\":\"x\"}");
        assert!(!ct.windows(6).any(|w| w == b"gender"));
        assert_eq!(decrypt(&key, &n, &ct).unwrap(), b"{\"gender\":\"x\"}");

        let (n2, _) = encrypt(&key, b"same");
        assert_ne!(n, n2, "nonces are fresh");

        let mut bad = ct.clone();
        bad[0] ^= 1;
        assert!(decrypt(&key, &n, &bad).is_none());
        assert!(decrypt(&ProfileKey([8; 32]), &n, &ct).is_none());
    }
}
