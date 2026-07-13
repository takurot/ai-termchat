use std::collections::{HashMap, HashSet};

use chacha20poly1305::aead::generic_array::GenericArray;
use chacha20poly1305::aead::{Aead, OsRng};
use chacha20poly1305::{ChaCha20Poly1305, Key, KeyInit, Nonce};
use message_io::network::{Endpoint, NetworkController, SendStatus};
use sha2::{Digest, Sha256};
use x25519_dalek::{EphemeralSecret, PublicKey};

use crate::message::NetMessage;

pub struct PeerSecureSession {
    pub send_cipher: ChaCha20Poly1305,
    pub recv_cipher: ChaCha20Poly1305,
    send_nonce: u64,
    seen_nonces: HashSet<u64>,
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
        let nonce_raw = u64::from_le_bytes(data[..8].try_into().unwrap());
        if !self.seen_nonces.insert(nonce_raw) {
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
        seen_nonces: HashSet::new(),
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
        seen_nonces: HashSet::new(),
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

/// Failure modes for [`encode_for_endpoint`].
#[derive(Debug)]
pub enum EncodeFrameError {
    /// `bincode` failed to serialize the (inner or outer) message.
    EncodeFailed(String),
    /// A session existed at the [`SecureState::has_session`] check but vanished
    /// before it could be used (e.g. raced with [`SecureState::remove`]).
    SessionLost,
}

impl std::fmt::Display for EncodeFrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncodeFrameError::EncodeFailed(detail) => {
                write!(f, "bincode encode failed: {detail}")
            }
            EncodeFrameError::SessionLost => write!(f, "secure session lost before encrypt"),
        }
    }
}

impl std::error::Error for EncodeFrameError {}

/// Encode `message` addressed to `endpoint` into wire bytes.
///
/// When a secure session exists for `endpoint`, the message is serialized,
/// encrypted with ChaCha20Poly1305, and wrapped in [`NetMessage::Secure`].
/// Otherwise it is serialized in plaintext (backward compatibility with peers
/// that have not completed a key exchange). Mirrors the encoding steps of
/// `Application::send_secure_to_peer` so the data plane and control plane share
/// one canonical encrypt-or-plaintext decision.
pub fn encode_for_endpoint(
    secure_state: &mut SecureState,
    endpoint: Endpoint,
    message: &NetMessage,
) -> Result<Vec<u8>, EncodeFrameError> {
    if !secure_state.has_session(endpoint) {
        return bincode::serde::encode_to_vec(message, bincode::config::legacy())
            .map_err(|e| EncodeFrameError::EncodeFailed(e.to_string()));
    }
    let serialized = bincode::serde::encode_to_vec(message, bincode::config::legacy())
        .map_err(|e| EncodeFrameError::EncodeFailed(e.to_string()))?;
    let ciphertext = secure_state
        .session_mut(endpoint)
        .ok_or(EncodeFrameError::SessionLost)?
        .encrypt(&serialized);
    bincode::serde::encode_to_vec(NetMessage::Secure(ciphertext), bincode::config::legacy())
        .map_err(|e| EncodeFrameError::EncodeFailed(e.to_string()))
}

/// Send `message` to every endpoint in `endpoints`, encrypting per-endpoint when
/// a secure session exists and falling back to plaintext otherwise.
///
/// Returns the same error shape (`Vec<(Endpoint, io::Error)>`) consumed by the
/// `Reportable` impl and `stringify_sendall_errors` in `util.rs`, so callers can
/// attribute failures to specific endpoints (e.g. `SendFile::failed_endpoints`
/// blacklisting). Per-endpoint encode/encrypt failures are attributed to the
/// offending endpoint rather than short-circuiting the whole broadcast.
pub fn send_secure_to_endpoints(
    network: &NetworkController,
    secure_state: &mut SecureState,
    endpoints: &[Endpoint],
    message: &NetMessage,
) -> Result<(), Vec<(Endpoint, std::io::Error)>> {
    let mut errors = Vec::new();
    for &endpoint in endpoints {
        let buf = match encode_for_endpoint(secure_state, endpoint, message) {
            Ok(buf) => buf,
            Err(err) => {
                errors.push((endpoint, std::io::Error::other(err.to_string())));
                continue;
            }
        };
        let status = network.send(endpoint, &buf);
        if status != SendStatus::Sent {
            errors.push((
                endpoint,
                std::io::Error::other(format!("send failed (status: {status:?})")),
            ));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::Chunk;

    fn fixture_endpoint(port: u16) -> Endpoint {
        // ResourceId encodes a transport tag in its bits; 130 maps to a valid
        // transport (same trick used by Application::inject_authenticated_peer_for_test).
        // Distinct ports yield distinct Endpoint values for the same resource.
        let id = message_io::network::ResourceId::from(130);
        let addr: std::net::SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
        Endpoint::from_listener(id, addr)
    }

    fn secure_state_with_session(endpoint: Endpoint) -> SecureState {
        let pending_a = generate_key_exchange();
        let pending_b = generate_key_exchange();
        let a_public = pending_a.public.to_bytes();
        let b_public = pending_b.public.to_bytes();
        let session = complete_key_exchange_as_initiator(pending_a.secret, &b_public).unwrap();
        // Keep the responder side referenced so the initiator session is valid.
        let _responder = complete_key_exchange_as_responder(pending_b.secret, &a_public).unwrap();
        let mut state = SecureState::default();
        state.sessions.insert(endpoint, session);
        state
    }

    #[test]
    fn encode_for_endpoint_encrypts_userdata_when_session_exists() {
        let endpoint = fixture_endpoint(0);
        let mut secure_state = secure_state_with_session(endpoint);
        let msg = NetMessage::UserData("secret.bin".into(), Chunk::Data(vec![1, 2, 3, 4, 5]));

        let encoded = encode_for_endpoint(&mut secure_state, endpoint, &msg).expect("encode ok");

        let decoded = crate::encoder::decode(&encoded).expect("must decode as NetMessage");
        match decoded {
            NetMessage::Secure(_) => { /* expected: wrapped in Secure envelope */ }
            other => panic!("expected NetMessage::Secure, got {other:?}"),
        }
    }

    #[test]
    fn encode_for_endpoint_emits_plaintext_when_no_session() {
        let endpoint_with = fixture_endpoint(0);
        let endpoint_without = fixture_endpoint(1);
        let mut secure_state = secure_state_with_session(endpoint_with);
        let msg = NetMessage::UserData("plain.bin".into(), Chunk::Data(vec![9, 9, 9]));

        let encoded =
            encode_for_endpoint(&mut secure_state, endpoint_without, &msg).expect("encode ok");

        let decoded = crate::encoder::decode(&encoded).expect("must decode as NetMessage");
        match decoded {
            NetMessage::UserData(name, Chunk::Data(bytes)) => {
                assert_eq!(name, "plain.bin");
                assert_eq!(bytes, vec![9, 9, 9]);
            }
            other => panic!("expected plaintext NetMessage::UserData, got {other:?}"),
        }
    }

    #[test]
    fn encode_for_endpoint_secure_output_differs_from_plaintext() {
        let endpoint_with = fixture_endpoint(0);
        let endpoint_without = fixture_endpoint(1);
        let mut secure_state = secure_state_with_session(endpoint_with);
        let msg = NetMessage::UserData("x".into(), Chunk::Data(vec![42; 32]));

        let encrypted = encode_for_endpoint(&mut secure_state, endpoint_with, &msg).unwrap();
        let plaintext = encode_for_endpoint(&mut secure_state, endpoint_without, &msg).unwrap();

        assert_ne!(
            encrypted, plaintext,
            "encrypted frame must not equal the plaintext serialization"
        );
        // The plaintext frame must not accidentally leak the raw chunk bytes via
        // a Secure-shaped envelope: it must decode to UserData, not Secure.
        match crate::encoder::decode(&plaintext).unwrap() {
            NetMessage::UserData(_, _) => {}
            other => panic!("plaintext path produced {other:?}, expected UserData"),
        }
    }

    #[test]
    fn encode_for_endpoint_encrypts_transfer_offer_when_session_exists() {
        let endpoint = fixture_endpoint(0);
        let mut secure_state = secure_state_with_session(endpoint);
        let msg = NetMessage::TransferOffer {
            file_name: "report.pdf".into(),
            file_size: 4096,
            sender: "alice".into(),
        };

        let encoded = encode_for_endpoint(&mut secure_state, endpoint, &msg).expect("encode ok");
        match crate::encoder::decode(&encoded).expect("must decode") {
            NetMessage::Secure(_) => {}
            other => panic!("expected NetMessage::Secure for TransferOffer, got {other:?}"),
        }
    }

    #[test]
    fn encode_frame_error_variants_display_for_io_bridge() {
        assert!(EncodeFrameError::EncodeFailed("boom".into())
            .to_string()
            .contains("bincode encode failed: boom"));
        assert!(EncodeFrameError::SessionLost.to_string().contains("session lost"));
    }

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

    #[test]
    fn replayed_nonce_is_rejected() {
        let pending_a = generate_key_exchange();
        let pending_b = generate_key_exchange();

        let a_public_bytes: [u8; 32] = pending_a.public.to_bytes();
        let b_public_bytes: [u8; 32] = pending_b.public.to_bytes();

        let mut session_a =
            complete_key_exchange_as_initiator(pending_a.secret, &b_public_bytes).unwrap();
        let mut session_b =
            complete_key_exchange_as_responder(pending_b.secret, &a_public_bytes).unwrap();

        let ct = session_a.encrypt(b"hello");
        let first = session_b.decrypt(&ct);
        assert!(first.is_some(), "first decrypt should succeed");

        let second = session_b.decrypt(&ct);
        assert!(second.is_none(), "replayed ciphertext must be rejected");
    }
}
