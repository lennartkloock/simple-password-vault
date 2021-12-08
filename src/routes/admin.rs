use crate::database::Password;
use crate::routes::{FlashContext, GeneralContext, VaultResponse};
use crate::sessions::{SafeSessionManager, TokenAuthResult, WithCookie};
use crate::{templates, VaultConfig, VaultDb};
use rocket::{form, http, request};

pub fn get_routes() -> Vec<rocket::Route> {
    rocket::routes![admin, add_password, add_password_submit]
}

#[derive(Default, serde::Serialize)]
struct AdminContext {
    general: GeneralContext,
    passwords: Vec<Password>,
}

impl AdminContext {
    fn with_config(mut self, config: &VaultConfig) -> Self {
        self.general = GeneralContext::from(config);
        self
    }

    fn with_passwords(mut self, passwords: Vec<Password>) -> Self {
        self.passwords = passwords;
        self
    }
}

#[rocket::get("/admin")]
async fn admin(
    auth: TokenAuthResult<WithCookie>,
    config: &rocket::State<VaultConfig>,
    session_manager: &rocket::State<SafeSessionManager>,
    database: &rocket::State<VaultDb>,
) -> VaultResponse<templates::Template> {
    if let Ok(token) = auth {
        if session_manager
            .lock()
            .await
            .is_admin_session(token.token())
            .unwrap_or(false)
        {
            let mut context = AdminContext::default().with_config(config);
            if let Ok(passwords) = database.fetch_all_password(false).await {
                context = context.with_passwords(passwords);
            }
            VaultResponse::Ok(templates::Template::render("admin", context))
        } else {
            VaultResponse::Err(http::Status::Unauthorized)
        }
    } else {
        VaultResponse::redirect_to(rocket::uri!(super::authentication::login))
    }
}

#[rocket::get("/admin/add-password")]
async fn add_password(
    auth: TokenAuthResult<WithCookie>,
    config: &rocket::State<VaultConfig>,
    session_manager: &rocket::State<SafeSessionManager>,
    flash: Option<request::FlashMessage<'_>>,
) -> VaultResponse<templates::Template> {
    if let Ok(token) = auth {
        if session_manager
            .lock()
            .await
            .is_admin_session(token.token())
            .unwrap_or(false)
        {
            VaultResponse::Ok(templates::Template::render(
                "add-password",
                FlashContext::default()
                    .with_config(config)
                    .with_optional_flash(flash),
            ))
        } else {
            VaultResponse::Err(http::Status::Unauthorized)
        }
    } else {
        VaultResponse::redirect_to(rocket::uri!(super::authentication::login))
    }
}

#[derive(rocket::FromForm)]
struct AddPasswordData<'a> {
    name: &'a str,
    password: &'a str,
    admin: bool,
}

#[rocket::post("/admin/add-password", data = "<form>")]
async fn add_password_submit(
    auth: TokenAuthResult<WithCookie>,
    session_manager: &rocket::State<SafeSessionManager>,
    database: &rocket::State<VaultDb>,
    form: form::Form<AddPasswordData<'_>>,
) -> VaultResponse<()> {
    if let Ok(token) = auth {
        if session_manager
            .lock()
            .await
            .is_admin_session(token.token())
            .unwrap_or(false)
        {
            match database
                .insert_password(form.name, form.password, form.admin)
                .await
            {
                Ok(_) => VaultResponse::redirect_to(rocket::uri!(admin)),
                Err(sqlx::Error::Database(e)) => {
                    VaultResponse::flash_error_redirect_to(rocket::uri!(add_password), e.message())
                }
                Err(_) => VaultResponse::Err(http::Status::InternalServerError),
            }
        } else {
            VaultResponse::Err(http::Status::Unauthorized)
        }
    } else {
        VaultResponse::redirect_to(rocket::uri!(super::authentication::login))
    }
}
