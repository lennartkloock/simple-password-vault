use sqlx::mysql;

pub struct VaultDb(mysql::MySqlPool);

pub async fn init(db_url: &str) -> Result<VaultDb, sqlx::Error> {
    mysql::MySqlPoolOptions::new()
        .connect(db_url)
        .await
        .map(VaultDb)
}
