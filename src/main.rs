use rocket::fairing;
use rocket::serde;

mod database;
mod routes;

#[derive(serde::Deserialize)]
pub struct VaultConfig {
    db_url: String,
}

#[rocket::main]
async fn main() {
    let rocket = rocket::build()
        .attach(fairing::AdHoc::config::<VaultConfig>())
        .mount("/", routes::get_routes());
    match rocket.figment().extract::<VaultConfig>() {
        Ok(config) => {
            match database::init(&config.db_url).await {
                Ok(db) => {
                    rocket.manage(db).launch().await.expect("Rocket blew up at launch (⩾﹏⩽)");
                }
                Err(e) => log::error!("An error occurred while initializing the database: {}\n\
                    Please make sure your MySQL/MariaDB server is responding at the specified url", e),
            }
        }
        Err(e) => log::error!("An error occurred while reading the config: {}", e),
    }
}
