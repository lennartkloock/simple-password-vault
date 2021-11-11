use crate::VaultConfig;

pub fn get_routes() -> Vec<rocket::Route> {
    rocket::routes![login, vault]
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
fn login(config: &rocket::State<VaultConfig>) -> rocket_dyn_templates::Template {
    rocket_dyn_templates::Template::render("login", GeneralContext::from(config.inner()))
}

#[rocket::get("/vault")]
fn vault(config: &rocket::State<VaultConfig>) -> rocket_dyn_templates::Template {
    rocket_dyn_templates::Template::render("vault", GeneralContext::from(config.inner()))
}
