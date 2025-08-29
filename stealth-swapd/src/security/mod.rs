use sha2::{Sha256, Digest};
use std::sync::Arc;
use secrecy::{Secret, SecretString, ExposeSecret};

pub struct KeyDerivation {
    encryption_key: Arc<Secret<[u8; 32]>>,
}

impl KeyDerivation {
    pub fn new(passphrase: SecretString) -> Self {
        let key = Self::derive_key_from_passphrase(passphrase.expose_secret());
        Self {
            encryption_key: Arc::new(Secret::new(key)),
        }
    }

    fn derive_key_from_passphrase(passphrase: &str) -> [u8; 32] {
        use sha2::{Digest};
        let mut hasher = Sha256::new();
        hasher.update(b"stealth-swap-encryption");
        hasher.update(passphrase.as_bytes());
        hasher.update(b"encryption-key");
        let output: [u8; 32] = hasher.finalize().into();
        output
    }

    pub fn generate_adaptor_secret() -> Secret<[u8; 32]> {
        let mut bytes = [0u8; 32];
        let mut rng = rand::thread_rng();
        rand::RngCore::fill_bytes(&mut rng, &mut bytes[..]);
        Secret::new(bytes)
    }

    pub fn derive_secret_hash(secret: &Secret<[u8; 32]>) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(secret.expose_secret());
        hasher.finalize().into()
    }

    pub fn generate_swap_id() -> [u8; 32] {
        let mut bytes = [0u8; 32];
        let mut rng = rand::thread_rng();
        rand::RngCore::fill_bytes(&mut rng, &mut bytes[..]);
        bytes
    }

    pub fn compute_adaptor_signature(
        message: &[u8],
        secret: &Secret<[u8; 32]>,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let keypair = ed25519_compact::KeyPair::from_seed(ed25519_compact::Seed::from_slice(secret.expose_secret())?);
        let signature = keypair.sk.sign(message, None);
        Ok(signature.to_vec())
    }

    pub fn secure_wipe<T>(secret: &mut T) {
        use std::mem::size_of_val;
        let bytes = unsafe {
            std::slice::from_raw_parts_mut(
                secret as *mut T as *mut u8,
                size_of_val(secret),
            )
        };
        for byte in bytes.iter_mut() {
            *byte = 0;
        }
    }
}