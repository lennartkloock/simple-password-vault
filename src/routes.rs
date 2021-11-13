use crate::{VaultConfig, VaultDb};
use rocket::response::Redirect;
use rocket::{http, response};
use rocket_dyn_templates as templates;

const SESSION_TOKEN_COOKIE: &str = "_session_token";

pub fn get_routes() -> Vec<rocket::Route> {
    rocket::routes![index, login, vault, vault_table_id]
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

#[rocket::get("/")]
async fn index(cookies: &http::CookieJar<'_>) -> response::Redirect {
    if cookies.get(SESSION_TOKEN_COOKIE).is_some() {
        Redirect::to(rocket::uri!(vault))
    } else {
        Redirect::to(rocket::uri!(login))
    }
}

#[rocket::get("/login")]
async fn login(
    config: &rocket::State<VaultConfig>,
    database: &rocket::State<VaultDb>,
) -> rocket_dyn_templates::Template {
    rocket::debug!("{:?}", database.fetch_all_password().await);
    rocket_dyn_templates::Template::render("login", GeneralContext::from(config.inner()))
}

#[rocket::get("/vault")]
async fn vault(config: &rocket::State<VaultConfig>) -> templates::Template {
    templates::Template::render("vault", GeneralContext::from(config.inner()))
}

#[rocket::get("/vault?<id>")]
async fn vault_table_id(config: &rocket::State<VaultConfig>, id: u32) -> templates::Template {
    templates::Template::render("table-not-found", GeneralContext::from(config.inner()))
}
