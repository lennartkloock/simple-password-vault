use sqlx::mysql;

pub struct VaultDb(mysql::MySqlPool);

pub async fn init(db_url: &str) -> Result<VaultDb, sqlx::Error> {
    mysql::MySqlPoolOptions::new()
        .connect(db_url)
        .await
        .map(VaultDb)
}

type QueryResult = sqlx::Result<sqlx::mysql::MySqlQueryResult>;

impl VaultDb {
    pub async fn setup(&self) -> sqlx::Result<()> {
        self.create_index_table().await.map(|qr| {
            log::debug!("Successfully created index table: {:?}", qr);
        })?;
        self.create_auth_table().await.map(|qr| {
            log::debug!("Successfully created auth table: {:?}", qr);
        })?;
        Ok(())
    }

    pub async fn create_index_table(&self) -> QueryResult {
        sqlx::query("CREATE TABLE IF NOT EXISTS vault_index (id int PRIMARY KEY AUTO_INCREMENT, table_name varchar(64) NOT NULL UNIQUE, ui_name varchar(64) NOT NULL UNIQUE)")
            .execute(&self.0)
            .await
    }

    pub async fn create_auth_table(&self) -> QueryResult {
        sqlx::query("CREATE TABLE IF NOT EXISTS vault_auth (id int PRIMARY KEY AUTO_INCREMENT, password_hash varchar(64) NOT NULL UNIQUE)")
            .execute(&self.0)
            .await
    }

    pub async fn create_vault_table(&self, table_name: &str) -> QueryResult {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS ? (id int PRIMARY KEY AUTO_INCREMENT, number varchar(256), password varchar(256))",
        )
        .bind(table_name)
        .execute(&self.0)
        .await
    }
}
