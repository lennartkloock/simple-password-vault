use crate::{VaultConfig, VaultDb};
use rocket::{form, http, response};
use rocket_dyn_templates as templates;

const SESSION_TOKEN_COOKIE: &str = "_session_token";

pub fn get_routes() -> Vec<rocket::Route> {
    rocket::routes![
        index,
        login,
        new_admin_password,
        new_admin_password_form,
        vault,
        vault_table_id
    ]
}

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

type ServerResponse<T> = Result<T, http::Status>;

#[rocket::get("/")]
async fn index(cookies: &http::CookieJar<'_>) -> response::Redirect {
    if cookies.get(SESSION_TOKEN_COOKIE).is_some() {
        response::Redirect::to(rocket::uri!(vault))
    } else {
        response::Redirect::to(rocket::uri!(login))
    }
}

#[rocket::get("/login")]
async fn login(
    config: &rocket::State<VaultConfig>,
    database: &rocket::State<VaultDb>,
) -> ServerResponse<Result<templates::Template, response::Redirect>> {
    if database
        .fetch_all_password(true)
        .await
        .map_err(|_| http::Status::InternalServerError)?
        .is_empty()
    {
        Ok(Err(response::Redirect::to(rocket::uri!(
            new_admin_password
        ))))
    } else {
        Ok(Ok(templates::Template::render(
            "login",
            GeneralContext::from(config.inner()),
        )))
    }
}

#[rocket::get("/new-admin-password")]
async fn new_admin_password(
    config: &rocket::State<VaultConfig>,
    database: &rocket::State<VaultDb>,
) -> ServerResponse<Result<templates::Template, response::Redirect>> {
    if database
        .fetch_all_password(true)
        .await
        .map_err(|_| http::Status::InternalServerError)?
        .is_empty()
    {
        Ok(Ok(templates::Template::render(
            "new-admin-password",
            GeneralContext::from(config.inner()),
        )))
    } else {
        Ok(Err(response::Redirect::to(rocket::uri!(login))))
    }
}

#[derive(rocket::FromForm)]
struct NewAdminPasswordData<'a> {
    password: &'a str,
    #[field(name = "password-confirm", validate = eq(self.password))]
    _password_confirm: &'a str,
}

#[rocket::post("/new-admin-password", data = "<form>")]
async fn new_admin_password_form(
    database: &rocket::State<VaultDb>,
    form: form::Form<form::Contextual<'_, NewAdminPasswordData<'_>>>,
) -> Result<response::Redirect, ServerResponse<String>> {
    if database
        .fetch_all_password(true)
        .await
        .map_err(|_| Err(http::Status::InternalServerError))?
        .is_empty()
    {
        if let Some(ref data) = form.value {
            database
                .insert_password(&data.password, true)
                .await
                .map(|_| response::Redirect::to(rocket::uri!(login)))
                .map_err(|_| Err(http::Status::InternalServerError))
        } else {
            Err(Ok(format!(
                "{}",
                form.context
                    .field_errors("password-confirm")
                    .fold(String::new(), |i, e| format!("{:?}\n{}", e, i))
            )))
        }
    } else {
        Ok(response::Redirect::to(rocket::uri!(login)))
    }
}

#[rocket::get("/vault")]
async fn vault(config: &rocket::State<VaultConfig>) -> templates::Template {
    templates::Template::render("vault", GeneralContext::from(config.inner()))
}

#[rocket::get("/vault?<id>")]
async fn vault_table_id(config: &rocket::State<VaultConfig>, id: u32) -> templates::Template {
    templates::Template::render("table-not-found", GeneralContext::from(config.inner()))
}
