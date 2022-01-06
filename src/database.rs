use crate::VaultConfig;
use rocket::fairing;
use sqlx::{mysql, Row};
use std::collections;

pub mod data;

pub use data::*;

const EXTRA_COLUMN_PREFIX: &str = "extra_";

/// This struct is a wrapper around the [`mysql::MySqlPool`].
/// It implements all functions and methods to interact with the vault database and its content.
pub struct VaultDb(
    /// The connection pool which is used to connect to the database
    mysql::MySqlPool,
);

type QueryResult = sqlx::Result<sqlx::mysql::MySqlQueryResult>;

// The following implementation of [`VaultDb`] is split into different impl blocks to improve readability.

// Setup
impl VaultDb {
    /// Returns a rocket fairing that will automatically setup this [`VaultDb`].
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

    /// Initializes this [`VaultDb`].
    ///
    /// This means:
    /// * Connects to the database server
    /// * Makes sure that all necessary tables were created by calling the internal [`VaultDb::setup`] method.
    ///
    /// # Arguments
    /// * `db_url`: The database url to which the connection will be established
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

    /// Makes sure that all necessary tables were created.
    ///
    /// In particular:
    /// * `table_index` (See: [`TableIndexEntry`])
    /// * `column_index` (See: [`ColumnIndexEntry`])
    /// * `auth` (See: [`Password`])
    ///
    /// This is an internal helper method which should only be invoked from the [`VaultDb::init`] function.
    async fn setup(&self) -> sqlx::Result<()> {
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
    /// Makes sure that the `table_index` table exists in the database by using the `CREATE TABLE IF NOT EXISTS` statement.
    ///
    /// If it does not exist, it will be created with the following columns:
    /// * `id int UNSIGNED PRIMARY KEY AUTO_INCREMENT`
    /// * `ui_name varchar(64) NOT NULL UNIQUE`
    pub async fn create_table_index(&self) -> QueryResult {
        log_and_return(
            sqlx::query("CREATE TABLE IF NOT EXISTS table_index (id int UNSIGNED PRIMARY KEY AUTO_INCREMENT, ui_name varchar(64) NOT NULL UNIQUE)")
                .execute(&self.0)
                .await
        )
    }

    /// Makes sure that the `column_index` table exists in the database by using the `CREATE TABLE IF NOT EXISTS` statement.
    ///
    /// If it does not exist, it will be created with the following columns:
    /// * `id int UNSIGNED PRIMARY KEY AUTO_INCREMENT`
    /// * `table_name varchar(64) NOT NULL`
    /// * `column_name varchar(64) NOT NULL`
    /// * `ui_name varchar(64) NOT NULL`
    /// * `required boolean NOT NULL`
    /// * `encrypted boolean NOT NULL`
    pub async fn create_column_index(&self) -> QueryResult {
        log_and_return(
            sqlx::query("CREATE TABLE IF NOT EXISTS column_index (id int UNSIGNED PRIMARY KEY AUTO_INCREMENT, table_name varchar(64) NOT NULL, column_name varchar(64) NOT NULL, ui_name varchar(64) NOT NULL, required boolean NOT NULL, encrypted boolean NOT NULL)")
                .execute(&self.0)
                .await
        )
    }

    /// Makes sure that the `auth` table exists in the database by using the `CREATE TABLE IF NOT EXISTS` statement.
    ///
    /// If it does not exist, it will be created with the following columns:
    /// * `id int UNSIGNED PRIMARY KEY AUTO_INCREMENT`
    /// * `name varchar(64) NOT NULL UNIQUE`
    /// * `password_hash varchar(64) NOT NULL UNIQUE`
    /// * `admin boolean NOT NULL`
    /// * `created datetime NOT NULL DEFAULT CURRENT_TIMESTAMP`
    pub async fn create_auth_table(&self) -> QueryResult {
        log_and_return(
            sqlx::query("CREATE TABLE IF NOT EXISTS auth (id int UNSIGNED PRIMARY KEY AUTO_INCREMENT, name varchar(64) NOT NULL UNIQUE, password_hash varchar(64) NOT NULL UNIQUE, admin boolean NOT NULL, created datetime NOT NULL DEFAULT CURRENT_TIMESTAMP)")
                .execute(&self.0)
                .await
        )
    }

    /// Creates a new vault table along with all required database entries.
    /// 1. Creates a new table index entry
    /// 2. Creates all new column index entries
    /// 3. Creates the vault table itself
    ///
    /// # Arguments
    /// * `ui_name`: The ui name of the table (e.g. `"Customer passwords"`)
    /// * `key_ui_name`: The ui name of the key column (e.g. `"CustomerID"`)
    /// * `password_ui_name`: The ui name of the password column (e.g. `"Password"`)
    /// * `extra`: An array of the ui names of all extra columns (e.g. `["Customer name", "Customer priority"]`)
    pub async fn create_vault_table(
        &self,
        ui_name: &str,
        key_ui_name: &str,
        password_ui_name: &str,
        extra: &[&str],
    ) -> sqlx::Result<u64> {
        //FIXME: Use a transaction

        // 1.
        let id = self
            .insert_table_index_entry(ui_name)
            .await?
            .last_insert_id();
        let table_name = gen_vault_table_name(id);

        // 2.
        //Using "key_" instead of "key" here because "key" is a reserved keyword in SQL
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

        // 3.
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
    /// Creates a new table index entry in the `table_index` database table.
    ///
    /// # Arguments
    /// * `ui_name`: The new table's ui name
    ///
    /// This is an internal helper method.
    ///
    /// *See also: [`TableIndexEntry`]*
    async fn insert_table_index_entry(&self, ui_name: &str) -> QueryResult {
        log_and_return(
            sqlx::query("INSERT INTO table_index (ui_name) VALUES (?)")
                .bind(ui_name)
                .execute(&self.0)
                .await,
        )
    }

    /// Creates a new column index entry in the `column_index` database table.
    ///
    /// # Arguments
    /// * `table_name`: The name of the table in which this column is located
    /// * `column_name`: The name of the column for which this index entry is created (This is the actual colum name, do not confuse with ui name!)
    /// * `ui_name`: The ui name of the column for which this index entry is created
    /// * `required`: If this column will be required to fill out when creating data
    /// * `required`: If this column's data should be encrypted
    ///
    /// This is an internal helper method.
    ///
    /// *See also: [`ColumnIndexEntry`]*
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

    /// Inserts new data into a vault table.
    ///
    /// # Arguments
    /// * `table_id`: The id of the vault table in which the data will be inserted
    /// * `data`: The data represented by a [`collections::HashMap<&str, &str>`]:
    /// The keys in that map should be the column names.
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

    /// Inserts a new access password into the `auth` database table.
    ///
    /// # Arguments
    /// * `name`: The password's name
    /// * `password`: The cleartext password
    /// * `admin`: Whether the new password will have admin privileges or not
    ///
    /// *See also: [`Password`]*
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
    /// Fetches the whole table index and returns it as a [`Vec`] of [`TableIndexEntry`]s.
    pub async fn fetch_table_index(&self) -> sqlx::Result<Vec<TableIndexEntry>> {
        log_and_return(
            sqlx::query_as::<_, TableIndexEntry>("SELECT * FROM table_index")
                .fetch_all(&self.0)
                .await,
        )
    }

    /// Fetches one table index entry by its `id` and returns it as a [`TableIndexEntry`].
    pub async fn fetch_table_index_entry(&self, id: u64) -> sqlx::Result<Option<TableIndexEntry>> {
        log_and_return(
            sqlx::query_as::<_, TableIndexEntry>("SELECT * FROM table_index WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.0)
                .await,
        )
    }

    /// Fetches all column index entries that are associated with the given `table_name` and returns them as a [`Vec`] of [`ColumnIndexEntry`]s.
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

    /// Fetches one column index entry by its `id` and returns it as a [`ColumnIndexEntry`].
    pub async fn fetch_column_index_by_id(&self, id: u64) -> sqlx::Result<Vec<ColumnIndexEntry>> {
        self.fetch_column_index(&gen_vault_table_name(id)).await
    }

    /// Fetches a table and its data by its `id` and returns it as a [`VaultTable`].
    ///
    /// # Arguments
    /// * `id`: The table's id
    /// * `query`: An optional query which is used to search in the table data and only return the data that matches the search term
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

    /// Fetches all access passwords and returns them as a [`Vec`] of [`Password`]s.
    ///
    /// If `only_admin` is set to `true`, this query will only return admin passwords.
    pub async fn fetch_all_password(&self, only_admin: bool) -> sqlx::Result<Vec<Password>> {
        log_and_return(
            sqlx::query_as::<_, Password>("SELECT * FROM auth WHERE IF(?, admin = 1, true)")
                .bind(only_admin)
                .fetch_all(&self.0)
                .await,
        )
    }

    /// Fetches an access password by its cleartext `password` and returns it as a [`Password`].
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
    /// Deletes a row in a vault table.
    ///
    /// # Arguments
    /// * `table_id`: The id of the table in which the row will be deleted
    /// * `row_id`: The id of the row which will be deleted
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

    /// Deletes an access password by its `id`.
    pub async fn delete_password(&self, id: u64) -> QueryResult {
        log_and_return(
            sqlx::query("DELETE FROM auth WHERE id = ?")
                .bind(id)
                .execute(&self.0)
                .await,
        )
    }

    /// Deletes all column index entries matching the given `table_id`.
    pub async fn delete_column_index(&self, table_id: u64) -> QueryResult {
        log_and_return(
            sqlx::query("DELETE FROM column_index WHERE table_name = ?")
                .bind(gen_vault_table_name(table_id))
                .execute(&self.0)
                .await,
        )
    }

    /// Deletes the table index entry with the given `table_id`.
    pub async fn delete_table_index_entry(&self, table_id: u64) -> QueryResult {
        log_and_return(
            sqlx::query("DELETE FROM table_index WHERE id = ?")
                .bind(table_id)
                .execute(&self.0)
                .await,
        )
    }

    /// Deletes a vault table by its `id`.
    ///
    /// This is done by:
    /// 1. Calling [`VaultDb::delete_column_index`]
    /// 2. Calling [`VaultDb::delete_table_index_entry`]
    /// 3. Deleting the vault table (with `DROP TABLE`)
    pub async fn delete_vault_table(&self, id: u64) -> QueryResult {
        // 1.
        self.delete_column_index(id).await?;
        // 2.
        self.delete_table_index_entry(id).await?;
        // 3.
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

/// This is an internal helper function to log database errors
fn log_and_return<T>(result: sqlx::Result<T>) -> sqlx::Result<T> {
    if let Err(ref e) = result {
        rocket::error!("Database error: {}", e);
    }
    result
}
