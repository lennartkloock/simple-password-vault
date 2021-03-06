use crate::VaultConfig;
use rocket::fairing;
use sqlx::{mysql, Row};
use std::collections;

pub mod data;

pub use data::*;

const EXTRA_COLUMN_PREFIX: &str = "extra_";

pub struct VaultDb(mysql::MySqlPool);

type QueryResult = sqlx::Result<sqlx::mysql::MySqlQueryResult>;

// Setup
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
}

// Create table statements
impl VaultDb {
    pub async fn create_table_index(&self) -> QueryResult {
        log_and_return(
            sqlx::query("CREATE TABLE IF NOT EXISTS table_index (id int UNSIGNED PRIMARY KEY AUTO_INCREMENT, ui_name varchar(64) NOT NULL UNIQUE)")
                .execute(&self.0)
                .await
        )
    }

    pub async fn create_column_index(&self) -> QueryResult {
        log_and_return(
            sqlx::query("CREATE TABLE IF NOT EXISTS column_index (id int UNSIGNED PRIMARY KEY AUTO_INCREMENT, table_name varchar(64) NOT NULL, column_name varchar(64) NOT NULL, ui_name varchar(64) NOT NULL, required boolean NOT NULL, encrypted boolean NOT NULL)")
                .execute(&self.0)
                .await
        )
    }

    pub async fn create_auth_table(&self) -> QueryResult {
        log_and_return(
            sqlx::query("CREATE TABLE IF NOT EXISTS auth (id int UNSIGNED PRIMARY KEY AUTO_INCREMENT, name varchar(64) NOT NULL UNIQUE, password_hash varchar(64) NOT NULL UNIQUE, admin boolean NOT NULL, created datetime NOT NULL DEFAULT CURRENT_TIMESTAMP)")
                .execute(&self.0)
                .await
        )
    }

    pub async fn create_vault_table(
        &self,
        ui_name: &str,
        key_ui_name: &str,
        password_ui_name: &str,
        extra: &[&str],
    ) -> sqlx::Result<u64> {
        //FIXME: Use a transaction
        let id = self
            .insert_table_index_entry(ui_name)
            .await?
            .last_insert_id();
        let table_name = gen_vault_table_name(id);

        self.insert_column_index_entry(&table_name, "key_", key_ui_name, true, false)
            .await?;
        self.insert_column_index_entry(&table_name, "password", password_ui_name, true, true)
            .await?;

        let extra_column_names: Vec<String> = (0..extra.len())
            .into_iter()
            .map(|x| format!("{}{}", EXTRA_COLUMN_PREFIX, x))
            .collect();
        for i in 0..extra.len() {
            self.insert_column_index_entry(
                &table_name,
                &extra_column_names[i],
                extra[i],
                false,
                false,
            )
            .await?;
        }

        let extra_columns = extra_column_names
            .iter()
            .fold(String::new(), |s, e| format!("{}, {} text", s, e));
        let statement = format!("CREATE TABLE {} (id int UNSIGNED PRIMARY KEY AUTO_INCREMENT, key_ varchar(256) NOT NULL, password text NOT NULL{})", table_name, extra_columns);
        log_and_return(sqlx::query(&statement).execute(&self.0).await)?;
        Ok(id)
    }
}

// Insert statements
impl VaultDb {
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
        required: bool,
        encrypted: bool,
    ) -> QueryResult {
        log_and_return(
            sqlx::query(
                "INSERT INTO column_index (table_name, column_name, ui_name, required, encrypted) VALUES (?, ?, ?, ?, ?)",
            )
                .bind(table_name)
                .bind(column_name)
                .bind(ui_name)
                .bind(required)
                .bind(encrypted)
                .execute(&self.0)
                .await,
        )
    }

    pub async fn insert_vault_data(
        &self,
        table_id: u64,
        data: collections::HashMap<&str, &str>,
    ) -> QueryResult {
        let table_name = gen_vault_table_name(table_id);
        let column_names = &data
            .iter()
            .fold(String::new(), |s, e| format!("{}, {}", s, e.0))[2..]; //Cut off preceding comma and space
        let value_placeholders = &", ?".repeat(data.len())[2..]; //Cut off preceding comma and space

        let statement = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table_name, column_names, value_placeholders
        );
        let mut query = sqlx::query(&statement);
        for datum in data {
            query = query.bind(datum.1);
        }
        log_and_return(query.execute(&self.0).await)
    }

    pub async fn insert_password(&self, name: &str, password: &str, admin: bool) -> QueryResult {
        log_and_return(
            sqlx::query(
                "INSERT INTO auth (name, password_hash, admin) VALUES (?, SHA2(?, 256), ?)",
            )
            .bind(name)
            .bind(password)
            .bind(admin)
            .execute(&self.0)
            .await,
        )
    }
}

// Fetch statements
impl VaultDb {
    pub async fn fetch_table_index(&self) -> sqlx::Result<Vec<TableIndexEntry>> {
        log_and_return(
            sqlx::query_as::<_, TableIndexEntry>("SELECT * FROM table_index")
                .fetch_all(&self.0)
                .await,
        )
    }

    pub async fn fetch_table_index_entry(&self, id: u64) -> sqlx::Result<Option<TableIndexEntry>> {
        log_and_return(
            sqlx::query_as::<_, TableIndexEntry>("SELECT * FROM table_index WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.0)
                .await,
        )
    }

    pub async fn fetch_column_index(
        &self,
        table_name: &str,
    ) -> sqlx::Result<Vec<ColumnIndexEntry>> {
        log_and_return(
            sqlx::query_as::<_, ColumnIndexEntry>(
                "SELECT * FROM column_index WHERE table_name = ?",
            )
            .bind(table_name)
            .fetch_all(&self.0)
            .await,
        )
    }

    pub async fn fetch_column_index_by_id(&self, id: u64) -> sqlx::Result<Vec<ColumnIndexEntry>> {
        self.fetch_column_index(&gen_vault_table_name(id)).await
    }

    pub async fn fetch_table(
        &self,
        id: u64,
        query: &Option<String>,
    ) -> sqlx::Result<Option<VaultTable>> {
        if let Some(table_index) = self.fetch_table_index_entry(id).await? {
            let table_name = gen_vault_table_name(table_index.id);
            let column_index = self.fetch_column_index(&table_name).await?;
            let statement = match query {
                Some(_) => format!(
                    "SELECT DISTINCT * FROM {} WHERE LOWER(key_) LIKE LOWER(?) ESCAPE '!'",
                    table_name
                ),
                None => format!("SELECT * FROM {}", table_name),
            };
            let data = log_and_return(
                sqlx::query(&statement)
                    .bind(query.as_ref().map(|s| gen_search_string(s)))
                    .fetch_all(&self.0)
                    .await,
            )?
            .into_iter()
            .map(|r| {
                let id: u64 = r.get("id");
                let cells = column_index
                    .iter()
                    .filter_map(|c| {
                        Some(TableCell {
                            data: r.try_get(&*c.column_name).ok()?,
                            encrypted: c.encrypted,
                        })
                    })
                    .collect();

                TableRow { id, cells }
            })
            .collect();
            sqlx::Result::Ok(Some(VaultTable {
                id,
                name: table_index.ui_name,
                columns: column_index,
                rows: data,
            }))
        } else {
            sqlx::Result::Ok(None)
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
}

// Delete Statements
impl VaultDb {
    pub async fn delete_vault_row(&self, table_id: u64, row_id: u64) -> QueryResult {
        log_and_return(
            sqlx::query(&format!(
                "DELETE FROM {} WHERE id = ?",
                gen_vault_table_name(table_id)
            ))
            .bind(row_id)
            .execute(&self.0)
            .await,
        )
    }

    pub async fn delete_password(&self, id: u64) -> QueryResult {
        log_and_return(
            sqlx::query("DELETE FROM auth WHERE id = ?")
                .bind(id)
                .execute(&self.0)
                .await,
        )
    }

    pub async fn delete_column_index(&self, table_id: u64) -> QueryResult {
        log_and_return(
            sqlx::query("DELETE FROM column_index WHERE table_name = ?")
                .bind(gen_vault_table_name(table_id))
                .execute(&self.0)
                .await,
        )
    }

    pub async fn delete_table_index_entry(&self, table_id: u64) -> QueryResult {
        log_and_return(
            sqlx::query("DELETE FROM table_index WHERE id = ?")
                .bind(table_id)
                .execute(&self.0)
                .await,
        )
    }

    pub async fn delete_vault_table(&self, id: u64) -> QueryResult {
        self.delete_column_index(id).await?;
        self.delete_table_index_entry(id).await?;
        log_and_return(
            sqlx::query(&format!("DROP TABLE {}", gen_vault_table_name(id)))
                .execute(&self.0)
                .await,
        )
    }
}

// From: https://stackoverflow.com/a/8248052/10772729
fn gen_search_string(query: &str) -> String {
    format!(
        "%{}%",
        query
            .replace("!", "!!")
            .replace("%", "%%")
            .replace("_", "!_")
            .replace("[", "![")
    )
}

fn gen_vault_table_name(id: u64) -> String {
    format!("vault_{}", id)
}

fn log_and_return<T>(result: sqlx::Result<T>) -> sqlx::Result<T> {
    if let Err(ref e) = result {
        rocket::error!("Database error: {}", e);
    }
    result
}
