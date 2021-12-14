use crate::VaultConfig;
use rocket::{http, request, response};

pub mod admin;
pub mod authentication;
pub mod export;
pub mod table_cud;
pub mod vault;

#[derive(serde::Serialize)]
struct GeneralContext {
    name: String,
    admin: bool,
}

impl Default for GeneralContext {
    fn default() -> Self {
        Self {
            name: "Password Vault".to_string(),
            admin: false,
        }
    }
}

impl From<&VaultConfig> for GeneralContext {
    fn from(config: &VaultConfig) -> Self {
        config
            .name
            .clone()
            .map(|name| GeneralContext {
                name,
                ..GeneralContext::default()
            })
            .unwrap_or_default()
    }
}

impl GeneralContext {
    fn with_admin(mut self, admin: bool) -> Self {
        self.admin = admin;
        self
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

    fn with_config(self, config: &VaultConfig) -> Self {
        self.with_general_context(GeneralContext::from(config))
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

    fn flash_success_redirect_to<U: TryInto<http::uri::Reference<'static>>, M: Into<String>>(
        uri: U,
        message: M,
    ) -> Self {
        Self::FlashRedirect(response::Flash::success(
            response::Redirect::to(uri),
            message,
        ))
    }

    fn redirect_to<U: TryInto<http::uri::Reference<'static>>>(uri: U) -> Self {
        Self::Redirect(response::Redirect::to(uri))
    }
}
