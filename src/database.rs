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
        let db: Self = log_and_return(
            mysql::MySqlPoolOptions::new()
                .connect(db_url)
                .await
                .map(Self),
        )?;
        db.setup().await?;
        Ok(db)
    }

    pub async fn setup(&self) -> sqlx::Result<()> {
        self.create_table_index().await.map(|qr| {
            rocket::debug!("Successfully created table index table: {:?}", qr);
        })?;
        self.create_column_index().await.map(|qr| {
            rocket::debug!("Successfully created column index table: {:?}", qr);
        })?;
        self.create_auth_table().await.map(|qr| {
            rocket::debug!("Successfully created auth table: {:?}", qr);
        })?;
        Ok(())
    }

    pub async fn create_table_index(&self) -> QueryResult {
        log_and_return(
            sqlx::query("CREATE TABLE IF NOT EXISTS table_index (id int UNSIGNED PRIMARY KEY AUTO_INCREMENT, ui_name varchar(64) NOT NULL UNIQUE)")
            .execute(&self.0)
            .await
        )
    }

    pub async fn create_column_index(&self) -> QueryResult {
        log_and_return(
            sqlx::query("CREATE TABLE IF NOT EXISTS column_index (id int UNSIGNED PRIMARY KEY AUTO_INCREMENT, table_name varchar(64) NOT NULL, column_name varchar(64) NOT NULL, ui_name varchar(64) NOT NULL UNIQUE)")
            .execute(&self.0)
            .await
        )
    }

    pub async fn create_auth_table(&self) -> QueryResult {
        log_and_return(
            sqlx::query("CREATE TABLE IF NOT EXISTS auth (id int UNSIGNED PRIMARY KEY AUTO_INCREMENT, password_hash varchar(64) NOT NULL UNIQUE, admin BOOLEAN NOT NULL)")
                .execute(&self.0)
                .await
        )
    }

    async fn insert_table_index_entry(&self, ui_name: &str) -> QueryResult {
        log_and_return(
            sqlx::query("INSERT INTO table_index (ui_name) VALUES (?)")
                .bind(ui_name)
                .execute(&self.0)
                .await,
        )
    }

    async fn insert_column_index_entry(
        &self,
        table_name: &str,
        column_name: &str,
        ui_name: &str,
    ) -> QueryResult {
        log_and_return(
            sqlx::query(
                "INSERT INTO column_index (table_name, column_name, ui_name) VALUES (?, ?, ?)",
            )
            .bind(table_name)
            .bind(column_name)
            .bind(ui_name)
            .execute(&self.0)
            .await,
        )
    }

    pub async fn create_vault_table(
        &self,
        ui_name: &str,
        extra: &[&str],
    ) -> (QueryResult, Option<u64>) {
        //TODO: Use a transaction
        let result = self.insert_table_index_entry(ui_name).await;
        if let Ok(r) = result {
            let id = r.last_insert_id();
            let table_name = format!("vault_{}", id);

            let extra_column_names: Vec<String> = (0..extra.len())
                .into_iter()
                .map(|x| format!("extra_{}", x))
                .collect();
            for i in 0..extra.len() {
                let result = self
                    .insert_column_index_entry(&table_name, &extra_column_names[i], extra[i])
                    .await;
                if result.is_err() {
                    return (result, None);
                }
            }

            let extra_columns = extra_column_names
                .iter()
                .fold(String::new(), |s, e| format!("{}, {} varchar(256)", s, e));
            let statement = format!("CREATE TABLE IF NOT EXISTS {} (id int UNSIGNED PRIMARY KEY AUTO_INCREMENT, number varchar(256), password varchar(256){})", table_name, extra_columns);
            (
                log_and_return(sqlx::query(&statement).execute(&self.0).await),
                Some(id),
            )
        } else {
            (result, None)
        }
    }

    pub async fn fetch_all_password(&self, only_admin: bool) -> sqlx::Result<Vec<Password>> {
        log_and_return(
            sqlx::query_as::<_, Password>("SELECT * FROM auth WHERE IF(?, admin = 1, true)")
                .bind(only_admin)
                .fetch_all(&self.0)
                .await,
        )
    }

    pub async fn fetch_password(&self, password: &str) -> sqlx::Result<Option<Password>> {
        log_and_return(
            sqlx::query_as::<_, Password>("SELECT * FROM auth WHERE password_hash = SHA2(?, 256)")
                .bind(password)
                .fetch_optional(&self.0)
                .await,
        )
    }

    pub async fn insert_password(&self, password: &str, admin: bool) -> QueryResult {
        log_and_return(
            sqlx::query("INSERT INTO auth (password_hash, admin) VALUES (SHA2(?, 256), ?)")
                .bind(password)
                .bind(admin)
                .execute(&self.0)
                .await,
        )
    }
}

fn log_and_return<T>(result: sqlx::Result<T>) -> sqlx::Result<T> {
    if let Err(ref e) = result {
        rocket::error!("Database error: {}", e);
    }
    result
}
