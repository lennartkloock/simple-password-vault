use crate::routes::{GeneralContext, VaultResponse};
use crate::sessions::{SafeSessionManager, SESSION_TOKEN_COOKIE};
use crate::{templates, VaultConfig, VaultDb};
use rocket::{form, http, response};

pub fn get_routes() -> Vec<rocket::Route> {
    rocket::routes![
        login,
        login_submit,
        new_admin_password,
        new_admin_password_form
    ]
}

enum AuthError {
    WrongPassword,
    PasswordTooShort,
}

impl From<AuthError> for String {
    fn from(error: AuthError) -> Self {
        match error {
            WrongPassword => "wrong_pass",
            PasswordTooShort => "short_pass",
        }.to_string()
    }
}

#[rocket::get("/login")]
async fn login(
    config: &rocket::State<VaultConfig>,
    database: &rocket::State<VaultDb>,
) -> VaultResponse<templates::Template> {
    match database.fetch_all_password(true).await {
        Ok(passwords) => {
            if passwords.is_empty() {
                VaultResponse::redirect_to(rocket::uri!(
                    new_admin_password
                ))
            }
            else {
                VaultResponse::Ok(templates::Template::render(
                    "login",
                    GeneralContext::from(config.inner()),
                ))
            }
        },
        Err(_) => VaultResponse::Err(http::Status::InternalServerError),
    }
}

#[derive(rocket::FromForm)]
struct LoginFormData<'a> {
    password: &'a str,
}

#[rocket::post("/login", data = "<form>")]
async fn login_submit(
    config: &rocket::State<VaultConfig>,
    database: &rocket::State<VaultDb>,
    session_manager: &rocket::State<SafeSessionManager>,
    cookies: &http::CookieJar<'_>,
    form: form::Form<LoginFormData<'_>>,
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
                VaultResponse::flash_error_redirect_to(rocket::uri!(login), AuthError::WrongPassword)
            }
        },
        Err(_) => VaultResponse::Err(http::Status::InternalServerError),
    }
}

#[rocket::get("/new-admin-password")]
async fn new_admin_password(
    config: &rocket::State<VaultConfig>,
    database: &rocket::State<VaultDb>,
) -> VaultResponse<templates::Template> {
    match database.fetch_all_password(true).await {
        Ok(passwords) => {
            if passwords.is_empty() {
                VaultResponse::Ok(templates::Template::render(
                    "new-admin-password",
                    GeneralContext::from(config.inner()),
                ))
            }
            else {
                VaultResponse::redirect_to(rocket::uri!(login))
            }
        },
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

// TODO: Fix this route
#[rocket::post("/new-admin-password", data = "<form>")]
async fn new_admin_password_form(
    database: &rocket::State<VaultDb>,
    form: form::Form<form::Contextual<'_, NewAdminPasswordData<'_>>>,
) -> VaultResponse<String> {
    match database.fetch_all_password(true).await {
        Ok(passwords) => {
            if passwords.is_empty() {
                if let Some(ref data) = form.value {
                    database
                        .insert_password(data.password, true)
                        .await
                        .map(|_| VaultResponse::redirect_to(rocket::uri!(login)))
                        .map_err(|_| VaultResponse::Err(http::Status::InternalServerError))
                } else {
                    Err(Ok(form
                        .context
                        .field_errors("password-confirm")
                        .fold(String::new(), |i, e| format!("{:?}\n{}", e, i))))
                }
            }
            else {
                VaultResponse::redirect_to(rocket::uri!(login))
            }
        },
        Err(_) => VaultResponse::Err(http::Status::InternalServerError),
    }
}
