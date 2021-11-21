use crate::VaultConfig;
use rocket::http;

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

#[derive(rocket::response::Responder)]
enum VaultResponse<T> {
    Ok(T),
    Redirect(response::Redirect),
    FlashRedirect(response::Flash<response::Redirect>),
    Err(http::Status),
}

impl<T> VaultResponse<T> {
    fn flash_error_redidrect_to<U: TryInto<http::uri::Reference<'static>>, M: Into<String>>(uri: U, message: M) -> Self<T> {
        Self::FlashRedirect(response::Flash:error(response::Redirect::to(uri), message))
    }

    fn redirect_to<U: TryInto<http::uri::Reference<'static>>>(uri: U) -> Self<T> {
        Self::Redirect(response::Redirect::to(uri))
    }
}
