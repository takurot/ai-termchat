use std::collections::HashMap;

use chacha20poly1305::aead::generic_array::GenericArray;
use chacha20poly1305::aead::{Aead, OsRng};
use chacha20poly1305::{ChaCha20Poly1305, Key, KeyInit, Nonce};
use message_io::network::Endpoint;
use sha2::{Digest, Sha256};
use x25519_dalek::{EphemeralSecret, PublicKey};

pub struct PeerSecureSession {
    pub send_cipher: ChaCha20Poly1305,
    pub recv_cipher: ChaCha20Poly1305,
    send_nonce: u64,
}

pub struct PendingKeyExchange {
    pub secret: EphemeralSecret,
    pub public: PublicKey,
}

impl PeerSecureSession {
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Vec<u8> {
        let nonce = make_nonce(self.send_nonce);
        self.send_nonce = self.send_nonce.wrapping_add(1);

        let ciphertext = self
            .send_cipher
            .encrypt(&nonce, plaintext)
            .expect("ChaCha20Poly1305 encrypt should not fail");

        let mut result = self.send_nonce.wrapping_sub(1).to_le_bytes().to_vec();
        result.extend_from_slice(&ciphertext);
        result
    }

    pub fn decrypt(&mut self, data: &[u8]) -> Option<Vec<u8>> {
        if data.len() < 8 {
            return None;
        }
        let nonce = make_nonce_from_bytes(&data[..8]);
        let ciphertext = &data[8..];
        self.recv_cipher.decrypt(&nonce, ciphertext).ok()
    }
}

fn make_nonce(counter: u64) -> Nonce {
    let mut nonce_bytes = [0u8; 12];
    nonce_bytes[..8].copy_from_slice(&counter.to_le_bytes());
    *GenericArray::from_slice(&nonce_bytes)
}

fn make_nonce_from_bytes(bytes: &[u8]) -> Nonce {
    let mut nonce_bytes = [0u8; 12];
    nonce_bytes[..8].copy_from_slice(bytes);
    *GenericArray::from_slice(&nonce_bytes)
}

fn derive_keys(shared_secret: &[u8; 32]) -> ([u8; 32], [u8; 32]) {
    let mut hasher = Sha256::new();
    hasher.update(b"triadchat-encryption-v1-send");
    hasher.update(shared_secret);
    let send_key: [u8; 32] = hasher.finalize_reset().into();

    hasher.update(b"triadchat-encryption-v1-recv");
    hasher.update(shared_secret);
    let recv_key: [u8; 32] = hasher.finalize().into();

    (send_key, recv_key)
}

pub fn complete_key_exchange_as_initiator(
    our_secret: EphemeralSecret,
    peer_public_bytes: &[u8; 32],
) -> Option<PeerSecureSession> {
    let peer_public = PublicKey::from(*peer_public_bytes);
    let shared_secret = our_secret.diffie_hellman(&peer_public);
    let (send_key, recv_key) = derive_keys(shared_secret.as_bytes());
    Some(PeerSecureSession {
        send_cipher: ChaCha20Poly1305::new(Key::from_slice(&send_key)),
        recv_cipher: ChaCha20Poly1305::new(Key::from_slice(&recv_key)),
        send_nonce: 0,
    })
}

pub fn complete_key_exchange_as_responder(
    our_secret: EphemeralSecret,
    peer_public_bytes: &[u8; 32],
) -> Option<PeerSecureSession> {
    let peer_public = PublicKey::from(*peer_public_bytes);
    let shared_secret = our_secret.diffie_hellman(&peer_public);
    let (send_key, recv_key) = derive_keys(shared_secret.as_bytes());
    Some(PeerSecureSession {
        send_cipher: ChaCha20Poly1305::new(Key::from_slice(&recv_key)),
        recv_cipher: ChaCha20Poly1305::new(Key::from_slice(&send_key)),
        send_nonce: 0,
    })
}

pub fn generate_key_exchange() -> PendingKeyExchange {
    let secret = EphemeralSecret::random_from_rng(OsRng);
    let public = PublicKey::from(&secret);
    PendingKeyExchange { secret, public }
}

#[derive(Default)]
pub struct SecureState {
    pub sessions: HashMap<Endpoint, PeerSecureSession>,
    pub pending_key_exchanges: HashMap<Endpoint, PendingKeyExchange>,
}

impl SecureState {
    pub fn session_mut(&mut self, endpoint: Endpoint) -> Option<&mut PeerSecureSession> {
        self.sessions.get_mut(&endpoint)
    }

    pub fn has_session(&self, endpoint: Endpoint) -> bool {
        self.sessions.contains_key(&endpoint)
    }

    pub fn remove(&mut self, endpoint: Endpoint) {
        self.sessions.remove(&endpoint);
        self.pending_key_exchanges.remove(&endpoint);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_round_trip() {
        let pending_a = generate_key_exchange();
        let pending_b = generate_key_exchange();

        let a_public_bytes: [u8; 32] = pending_a.public.to_bytes();
        let b_public_bytes: [u8; 32] = pending_b.public.to_bytes();

        let mut session_a =
            complete_key_exchange_as_initiator(pending_a.secret, &b_public_bytes).unwrap();
        let mut session_b =
            complete_key_exchange_as_responder(pending_b.secret, &a_public_bytes).unwrap();

        let plaintext = b"hello encrypted world";
        let ct = session_a.encrypt(plaintext);
        let decrypted = session_b.decrypt(&ct).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn encrypt_decrypt_both_directions() {
        let pending_a = generate_key_exchange();
        let pending_b = generate_key_exchange();

        let a_public_bytes: [u8; 32] = pending_a.public.to_bytes();
        let b_public_bytes: [u8; 32] = pending_b.public.to_bytes();

        let mut session_a =
            complete_key_exchange_as_initiator(pending_a.secret, &b_public_bytes).unwrap();
        let mut session_b =
            complete_key_exchange_as_responder(pending_b.secret, &a_public_bytes).unwrap();

        let msg1 = b"a to b";
        let ct1 = session_a.encrypt(msg1);
        let pt1 = session_b.decrypt(&ct1).unwrap();
        assert_eq!(pt1, msg1);

        let msg2 = b"b to a";
        let ct2 = session_b.encrypt(msg2);
        let pt2 = session_a.decrypt(&ct2).unwrap();
        assert_eq!(pt2, msg2);
    }

    #[test]
    fn wrong_key_fails() {
        let pending_a = generate_key_exchange();
        let pending_b = generate_key_exchange();
        let pending_c = generate_key_exchange(); // attacker

        let b_public_bytes: [u8; 32] = pending_b.public.to_bytes();
        let c_public_bytes: [u8; 32] = pending_c.public.to_bytes();

        let mut session_a =
            complete_key_exchange_as_initiator(pending_a.secret, &b_public_bytes).unwrap();
        let mut session_attacker =
            complete_key_exchange_as_responder(pending_c.secret, &c_public_bytes).unwrap();

        let ct = session_a.encrypt(b"secret");
        assert!(session_attacker.decrypt(&ct).is_none());
    }

    #[test]
    fn tampered_data_is_rejected() {
        let pending_a = generate_key_exchange();
        let pending_b = generate_key_exchange();

        let a_public_bytes: [u8; 32] = pending_a.public.to_bytes();
        let b_public_bytes: [u8; 32] = pending_b.public.to_bytes();

        let mut session_a =
            complete_key_exchange_as_initiator(pending_a.secret, &b_public_bytes).unwrap();
        let mut session_b =
            complete_key_exchange_as_responder(pending_b.secret, &a_public_bytes).unwrap();

        let mut ct = session_a.encrypt(b"secret");
        // flip a byte in the ciphertext
        if let Some(last) = ct.last_mut() {
            *last ^= 1;
        }
        assert!(session_b.decrypt(&ct).is_none());
    }
}
