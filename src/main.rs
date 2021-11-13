use crate::database::VaultDb;
use rocket::fairing;
use rocket::serde;

mod database;
mod routes;

#[derive(serde::Deserialize)]
pub struct VaultConfig {
    name: Option<String>,
    db_url: String,
    static_dir: String,
}

#[rocket::main]
async fn main() {
    let rocket = rocket::build()
        .attach(fairing::AdHoc::config::<VaultConfig>())
        .attach(rocket_dyn_templates::Template::fairing())
        .mount("/", routes::get_routes());
    match rocket.figment().extract::<VaultConfig>() {
        Ok(config) => match init_database(&config).await {
            Ok(db) => {
                rocket
                    .mount("/", rocket::fs::FileServer::from(&config.static_dir))
                    .manage(db)
                    .launch()
                    .await
                    .expect("Rocket blew up at launch (⩾﹏⩽)");
            }
            Err(e) => rocket::error!(
                "An error occurred while initializing the database: {}\n\
                    Please make sure your MySQL/MariaDB server is responding at the specified url",
                e
            ),
        },
        Err(e) => rocket::error!("An error occurred while reading the config: {}", e),
    }
}

async fn init_database(config: &VaultConfig) -> sqlx::Result<VaultDb> {
    let db = database::init(&config.db_url).await?;
    db.setup().await?;
    Ok(db)
}
