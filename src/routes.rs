use crate::{VaultConfig, VaultDb};

pub fn get_routes() -> Vec<rocket::Route> {
    rocket::routes![login, vault, vault_table_id]
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

#[rocket::get("/login")]
async fn login(
    config: &rocket::State<VaultConfig>,
    database: &rocket::State<VaultDb>,
) -> rocket_dyn_templates::Template {
    rocket::debug!("{:?}", database.fetch_all_password().await);
    rocket_dyn_templates::Template::render("login", GeneralContext::from(config.inner()))
}

#[rocket::get("/vault")]
async fn vault(config: &rocket::State<VaultConfig>) -> rocket_dyn_templates::Template {
    rocket_dyn_templates::Template::render("vault", GeneralContext::from(config.inner()))
}

#[rocket::get("/vault?<id>")]
async fn vault_table_id(
    config: &rocket::State<VaultConfig>,
    id: u32,
) -> rocket_dyn_templates::Template {
    rocket_dyn_templates::Template::render("table-not-found", GeneralContext::from(config.inner()))
}
