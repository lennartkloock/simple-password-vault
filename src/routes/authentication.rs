//! Contains all routes and types that are being used for authentication

use crate::routes::{FlashContext, VaultResponse};
use crate::sessions::{SafeSessionManager, SESSION_TOKEN_COOKIE};
use crate::{templates, VaultConfig, VaultDb};
use rocket::{form, http, request};

pub fn get_routes() -> Vec<rocket::Route> {
    rocket::routes![
        login,
        login_submit,
        logout_submit,
        new_admin_password,
        new_admin_password_form
    ]
}

#[rocket::get("/login")]
async fn login(
    config: &rocket::State<VaultConfig>,
    database: &rocket::State<VaultDb>,
    flash: Option<request::FlashMessage<'_>>,
) -> VaultResponse<templates::Template> {
    match database.fetch_all_password(true).await {
        Ok(passwords) => {
            if passwords.is_empty() {
                VaultResponse::redirect_to(rocket::uri!(new_admin_password))
            } else {
                let context = FlashContext::default()
                    .with_config(config)
                    .with_optional_flash(flash);
                VaultResponse::Ok(templates::Template::render("login", context))
            }
        }
        Err(_) => VaultResponse::Err(http::Status::InternalServerError),
    }
}

#[derive(rocket::FromForm)]
struct LoginFormData<'a> {
    password: &'a str,
}

#[rocket::post("/login", data = "<form>")]
async fn login_submit(
    form: form::Form<LoginFormData<'_>>,
    cookies: &http::CookieJar<'_>,
    config: &rocket::State<VaultConfig>,
    session_manager: &rocket::State<SafeSessionManager>,
    database: &rocket::State<VaultDb>,
) -> VaultResponse<()> {
    match database.fetch_password(form.password).await {
        Ok(password) => {
            if password.is_some() {
                let token = session_manager.lock().await.generate_session(
                    config.token_length.unwrap_or(32) as usize,
                    std::time::Duration::from_secs(config.token_validity_duration_secs),
                );
                cookies.add(
                    http::Cookie::build(SESSION_TOKEN_COOKIE, token.0)
                        .max_age(time::Duration::seconds(
                            config.token_validity_duration_secs as i64,
                        ))
                        .http_only(true)
                        .finish(),
                );
                VaultResponse::redirect_to(rocket::uri!(super::vault::vault))
            } else {
                VaultResponse::flash_error_redirect_to(
                    rocket::uri!(login),
                    "The given password is wrong, please try again",
                )
            }
        }
        Err(_) => VaultResponse::Err(http::Status::InternalServerError),
    }
}

#[rocket::post("/logout")]
async fn logout_submit(cookies: &http::CookieJar<'_>) -> VaultResponse<()> {
    cookies.remove(http::Cookie::named(SESSION_TOKEN_COOKIE));
    VaultResponse::flash_success_redirect_to(rocket::uri!(login), "Successfully logged out")
}

#[rocket::get("/new-admin-password")]
async fn new_admin_password(
    config: &rocket::State<VaultConfig>,
    database: &rocket::State<VaultDb>,
    flash: Option<request::FlashMessage<'_>>,
) -> VaultResponse<templates::Template> {
    match database.fetch_all_password(true).await {
        Ok(passwords) => {
            if passwords.is_empty() {
                let context = FlashContext::default()
                    .with_config(config)
                    .with_optional_flash(flash);
                VaultResponse::Ok(templates::Template::render("new-admin-password", context))
            } else {
                VaultResponse::redirect_to(rocket::uri!(login))
            }
        }
        Err(_) => VaultResponse::Err(http::Status::InternalServerError),
    }
}

#[derive(rocket::FromForm)]
struct NewAdminPasswordData<'a> {
    #[field(validate = len(8..))]
    password: &'a str,
    #[field(name = "password-confirm", validate = eq(self.password))]
    _password_confirm: &'a str,
}

#[rocket::post("/new-admin-password", data = "<form>")]
async fn new_admin_password_form(
    form: form::Form<form::Contextual<'_, NewAdminPasswordData<'_>>>,
    database: &rocket::State<VaultDb>,
) -> VaultResponse<String> {
    match database.fetch_all_password(true).await {
        Ok(passwords) => {
            if passwords.is_empty() {
                if let Some(ref data) = form.value {
                    if database.insert_password(data.password, true).await.is_ok() {
                        VaultResponse::redirect_to(rocket::uri!(login))
                    } else {
                        VaultResponse::Err(http::Status::InternalServerError)
                    }
                } else {
                    VaultResponse::flash_error_redirect_to(
                        rocket::uri!(new_admin_password),
                        form.context
                            .field_errors("password-confirm")
                            .fold(String::new(), |i, e| format!("{:?}\n{}", e.kind, i)), //TODO: Improve error handling
                    )
                }
            } else {
                VaultResponse::redirect_to(rocket::uri!(login))
            }
        }
        Err(_) => VaultResponse::Err(http::Status::InternalServerError),
    }
}
