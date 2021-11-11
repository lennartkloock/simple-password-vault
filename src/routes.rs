use crate::VaultConfig;

pub fn get_routes() -> Vec<rocket::Route> {
    rocket::routes![login, vault]
}

#[derive(serde::Serialize)]
struct GeneralContext {
    name: String,
}

#[rocket::get("/login")]
fn login(config: &rocket::State<VaultConfig>) -> rocket_dyn_templates::Template {
    rocket_dyn_templates::Template::render(
        "login",
        GeneralContext {
            name: config
                .name
                .clone()
                .unwrap_or(String::from("Password Vault")),
        },
    )
}

#[rocket::get("/vault")]
fn vault(config: &rocket::State<VaultConfig>) -> rocket_dyn_templates::Template {
    rocket_dyn_templates::Template::render(
        "vault",
        GeneralContext {
            name: config
                .name
                .clone()
                .unwrap_or(String::from("Password Vault")),
        },
    )
}
