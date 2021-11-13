use crate::VaultConfig;

const SESSION_TOKEN_COOKIE: &str = "_session_token";

pub mod authentication;
pub mod vault;

#[derive(serde::Serialize)]
struct GeneralContext {
    name: String,
}

impl Default for GeneralContext {
    fn default() -> Self {
        Self {
            name: "Password Vault".to_string(),
        }
    }
}

impl From<&VaultConfig> for GeneralContext {
    fn from(config: &VaultConfig) -> Self {
        config
            .name
            .clone()
            .map(|name| GeneralContext { name })
            .unwrap_or_default()
    }
}
