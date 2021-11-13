use crate::database::VaultDb;
use rocket::{fairing, fs, serde};

mod database;
mod routes;

#[derive(Debug, serde::Deserialize)]
pub struct VaultConfig {
    name: Option<String>,
    db_url: String,
    static_dir: String,
}

#[rocket::main]
async fn main() {
    let mut rocket = rocket::build()
        .attach(fairing::AdHoc::config::<VaultConfig>())
        .attach(VaultDb::fairing().await)
        .attach(rocket_dyn_templates::Template::fairing())
        .mount("/", routes::get_routes());

    let static_dir: Option<String> = rocket.state::<VaultConfig>().map(|c| c.static_dir.clone());
    if let Some(static_dir) = static_dir {
        rocket = rocket.mount("/", fs::FileServer::from(&static_dir));
    } else {
        rocket::error!("An error occurred while reading the config")
    }

    rocket
        .launch()
        .await
        .expect("Rocket blew up at launch (⩾﹏⩽)");
}
