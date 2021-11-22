use crate::VaultConfig;
use rocket::{http, request, response};

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

#[derive(Default, serde::Serialize)]
struct FlashContext {
    general: GeneralContext,
    kind: Option<String>,
    message: Option<String>,
}

impl FlashContext {
    fn with_general_context(mut self, general: GeneralContext) -> Self {
        self.general = general;
        self
    }

    fn with_optional_flash(mut self, flash: Option<request::FlashMessage>) -> Self {
        self.kind = flash.as_ref().map(|f| f.kind().to_string());
        self.message = flash.as_ref().map(|f| f.message().to_string());
        self
    }
}

#[derive(response::Responder)]
enum VaultResponse<T> {
    Ok(T),
    Redirect(response::Redirect),
    FlashRedirect(response::Flash<response::Redirect>),
    Err(http::Status),
}

impl<T> VaultResponse<T> {
    fn flash_error_redirect_to<U: TryInto<http::uri::Reference<'static>>, M: Into<String>>(
        uri: U,
        message: M,
    ) -> Self {
        Self::FlashRedirect(response::Flash::error(response::Redirect::to(uri), message))
    }

    fn redirect_to<U: TryInto<http::uri::Reference<'static>>>(uri: U) -> Self {
        Self::Redirect(response::Redirect::to(uri))
    }
}
