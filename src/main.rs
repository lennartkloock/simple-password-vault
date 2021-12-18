//! # 🔐 Simple Password Vault
//! 🚧 In development 🚧
//!
//! ## License
//! This software is licensed under the terms of the MIT license

use crate::database::VaultDb;
use rocket::{fairing, fs, serde};
use rocket_dyn_templates as templates;

mod crypt;
mod database;
mod routes;
mod sessions;

#[derive(Debug, serde::Deserialize)]
pub struct VaultConfig {
    #[serde(default = "default_name")]
    name: String,
    db_url: String,
    #[serde(default = "default_static_dir")]
    static_dir: String,
    #[serde(default = "default_token_length")]
    token_length: u32,
    #[serde(default = "default_token_validity")]
    token_validity_duration_secs: u64,
    #[serde(default = "default_public_key")]
    public_key_path: String,
    #[serde(default = "default_private_key")]
    private_key_path: String,
}

fn default_name() -> String {
    "Password Vault".to_string()
}
fn default_static_dir() -> String {
    "public/static".to_string()
}
fn default_token_length() -> u32 {
    32
}
fn default_token_validity() -> u64 {
    86400
}
fn default_public_key() -> String {
    "keys/rsapubkey.pem".to_string()
}
fn default_private_key() -> String {
    "keys/rsakey.pem".to_string()
}

#[rocket::main]
async fn main() {
    let rocket = rocket::build()
        .attach(fairing::AdHoc::config::<VaultConfig>())
        .attach(VaultDb::fairing().await)
        .attach(crypt::KeyPair::fairing().await)
        .attach(sessions::SessionManager::fairing())
        .attach(templates::Template::fairing())
        .mount("/", routes::admin::get_routes())
        .mount("/", routes::authentication::get_routes())
        .mount("/", routes::xport::get_routes())
        .mount("/", routes::table_cud::get_routes())
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
