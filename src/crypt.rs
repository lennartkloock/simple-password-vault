use crate::VaultConfig;
use rocket::{fairing, tokio};
use rsa::pkcs1::FromRsaPrivateKey;
use rsa::pkcs8::FromPublicKey;
use rsa::PublicKey;
use std::{error, string};

pub struct KeyPair(pub rsa::RsaPublicKey, pub rsa::RsaPrivateKey);

impl KeyPair {
    async fn new(
        public_key_path: &str,
        private_key_path: &str,
    ) -> Result<Self, Box<dyn error::Error>> {
        let public = tokio::fs::read_to_string(public_key_path).await?;
        let private = tokio::fs::read_to_string(private_key_path).await?;
        Ok(Self(
            rsa::RsaPublicKey::from_public_key_pem(&public)?,
            rsa::RsaPrivateKey::from_pkcs1_pem(&private)?,
        ))
    }

    pub async fn fairing() -> impl fairing::Fairing {
        fairing::AdHoc::try_on_ignite("Private key", |rocket| async move {
            if let Some(config) = rocket.state::<VaultConfig>() {
                match Self::new(&config.public_key_path, &config.private_key_path).await {
                    Ok(slf) => Ok(rocket.manage(slf)),
                    Err(e) => {
                        rocket::error!("Error while parsing private key: {}", e);
                        Err(rocket)
                    }
                }
            } else {
                Err(rocket)
            }
        })
    }

    pub fn encrypt_string_to_hex(&self, s: &str) -> rsa::errors::Result<String> {
        let mut rng = rand::rngs::OsRng;
        self.0
            .encrypt(
                &mut rng,
                rsa::PaddingScheme::new_pkcs1v15_encrypt(),
                s.as_bytes(),
            )
            .map(hex::encode)
    }

    pub fn decrypt_string_from_hex(&self, hex: &str) -> Result<String, DecryptionError> {
        String::from_utf8(
            self.1
                .decrypt(
                    rsa::PaddingScheme::new_pkcs1v15_encrypt(),
                    &hex::decode(hex).map_err(DecryptionError::ParseHex)?,
                )
                .map_err(DecryptionError::Rsa)?,
        )
        .map_err(DecryptionError::ParseString)
    }
}

pub enum DecryptionError {
    ParseHex(hex::FromHexError),
    Rsa(rsa::errors::Error),
    ParseString(string::FromUtf8Error),
}
