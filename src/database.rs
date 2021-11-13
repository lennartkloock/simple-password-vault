use crate::VaultConfig;
use rocket::fairing;
use sqlx::mysql;

pub struct VaultDb(mysql::MySqlPool);

#[derive(Debug, sqlx::FromRow)]
pub struct Password {
    id: u32,
    password_hash: String,
    admin: bool,
}

type QueryResult = sqlx::Result<sqlx::mysql::MySqlQueryResult>;

impl VaultDb {
    pub async fn fairing() -> impl fairing::Fairing {
        fairing::AdHoc::try_on_ignite("Database", |rocket| async move {
            if let Some(config) = rocket.state::<VaultConfig>() {
                match VaultDb::init(&config.db_url).await {
                    Ok(db) => return Ok(rocket.manage(db)),
                    Err(e) => {
                        rocket::error!(
                            "An error occurred while initializing the database: {}\n\
                                Please make sure your MySQL/MariaDB server is responding at the specified url",
                            e
                        );
                    }
                }
            }
            Err(rocket)
        })
    }

    pub async fn init(db_url: &str) -> Result<Self, sqlx::Error> {
        let db: Self = mysql::MySqlPoolOptions::new()
            .connect(db_url)
            .await
            .map(Self)?;
        db.setup().await?;
        Ok(db)
    }

    pub async fn setup(&self) -> sqlx::Result<()> {
        self.create_index_table().await.map(|qr| {
            rocket::debug!("Successfully created index table: {:?}", qr);
        })?;
        self.create_auth_table().await.map(|qr| {
            rocket::debug!("Successfully created auth table: {:?}", qr);
        })?;
        Ok(())
    }

    pub async fn create_index_table(&self) -> QueryResult {
        sqlx::query("CREATE TABLE IF NOT EXISTS vault_index (id int UNSIGNED PRIMARY KEY AUTO_INCREMENT, table_name varchar(64) NOT NULL UNIQUE, ui_name varchar(64) NOT NULL UNIQUE)")
            .execute(&self.0)
            .await
    }

    pub async fn create_auth_table(&self) -> QueryResult {
        sqlx::query("CREATE TABLE IF NOT EXISTS vault_auth (id int UNSIGNED PRIMARY KEY AUTO_INCREMENT, password_hash varchar(64) NOT NULL UNIQUE, admin BOOLEAN NOT NULL)")
            .execute(&self.0)
            .await
    }

    pub async fn create_vault_table(&self, table_name: &str) -> QueryResult {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS ? (id int UNSIGNED PRIMARY KEY AUTO_INCREMENT, number varchar(256), password varchar(256))",
        )
        .bind(table_name)
        .execute(&self.0)
        .await
    }

    pub async fn fetch_all_password(&self, only_admin: bool) -> sqlx::Result<Vec<Password>> {
        sqlx::query_as::<_, Password>("SELECT * FROM vault_auth WHERE IF(?, admin = 1, true)")
            .bind(only_admin)
            .fetch_all(&self.0)
            .await
    }

    pub async fn insert_password(&self, password: &str, admin: bool) -> QueryResult {
        sqlx::query("INSERT INTO vault_auth (password_hash, admin) VALUES (SHA2(?, 256), ?)")
            .bind(password)
            .bind(admin)
            .execute(&self.0)
            .await
    }
}
