use crate::database::VaultDb;
use rocket::{fairing, fs, serde};
use rocket_dyn_templates as templates;

mod database;
mod routes;
mod sessions;

#[derive(Debug, serde::Deserialize)]
pub struct VaultConfig {
    name: Option<String>,
    db_url: String,
    static_dir: String,
    token_validity_duration_secs: u64,
}

#[rocket::main]
async fn main() {
    let rocket = rocket::build()
        .attach(fairing::AdHoc::config::<VaultConfig>())
        .attach(VaultDb::fairing().await)
        .attach(sessions::SessionManager::fairing())
        .attach(templates::Template::fairing())
        .mount("/", routes::authentication::get_routes())
        .mount("/", routes::vault::get_routes());

    match rocket.figment().extract::<VaultConfig>() {
        Ok(config) => {
            rocket
                .mount("/", fs::FileServer::from(&config.static_dir))
                .launch()
                .await
                .expect("Rocket blew up at launch (⩾﹏⩽)");
        }
        Err(e) => rocket::error!("An error occurred while reading the config: {}", e),
    }
}
