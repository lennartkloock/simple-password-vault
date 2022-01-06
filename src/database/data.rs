//! This module contains all data types which are used to interact with the database

use crate::crypt;

/// This struct represents an access password which can be used to log in.
///
/// Not to confuse with a saved password (represented by [`TableCell`]).
#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct Password {
    /// The password's id
    pub id: u64,
    /// The password's name, which can be displayed in the UI
    pub name: String,
    /// The password's hash
    pub password_hash: String,
    /// If this password can be used to log in as admin
    pub admin: bool,
    /// The password's creation date and time (represented by [`chrono::DateTime<chrono::Utc>`])
    pub created: chrono::DateTime<chrono::Utc>,
}

/// This struct represents an entry in the table index.
/// Each password table in the vault needs to have an associated table index entry to be recognized.
/// The index entries are persisted in the `table_index` database table.
#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct TableIndexEntry {
    /// The table's id
    ///
    /// The table with the id `x` is called `vault_x` in the database.
    pub id: u64,
    /// The table's ui name
    pub ui_name: String,
}

/// This struct represents an entry in the column index.
/// Each column in a password table needs an associated column index entry to be recognized.
/// The index entries are persisted in the `column_index` database table.
#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct ColumnIndexEntry {
    /// The entry's id
    pub id: u64,
    /// The column's table name
    ///
    /// *See also: [`TableIndexEntry`]'s `id` field*
    pub table_name: String,
    /// The column's name in the database (Do not confuse with `ui_name`!)
    pub column_name: String,
    /// The column's ui name
    pub ui_name: String,
    /// If this column is required to be filled out when adding data to the password table
    pub required: bool,
    /// If this column's data should be encrypted
    pub encrypted: bool,
}

/// This struct represents a password vault table.
/// This vault table with the id `x` is always associated with the database table called `vault_x`.
#[derive(Default, Debug, serde::Serialize)]
pub struct VaultTable {
    /// The table's id
    ///
    /// *See also: [`TableIndexEntry`]'s `id` field*
    pub id: u64,
    /// The table's ui name
    ///
    /// *See also: [`TableIndexEntry`]'s `ui_name` field*
    pub name: String,
    /// The table's columns
    pub columns: Vec<ColumnIndexEntry>,
    /// The table's rows
    pub rows: Vec<TableRow>,
}

/// This struct represents a table row.
/// It has no database table directly associated with it.
#[derive(Default, Debug, serde::Serialize)]
pub struct TableRow {
    /// The row's id
    pub id: u64,
    /// The row's cells
    pub cells: Vec<TableCell>,
}

/// This struct represents a table cell.
/// It has no database table directly associated with it.
#[derive(Default, Debug, serde::Serialize)]
pub struct TableCell {
    /// The cell's data, represented as a string
    pub data: String,
    /// If the cell's `data` field is encrypted
    pub encrypted: bool,
}

impl VaultTable {
    /// Exports this vault table to a string formatted as CSV.
    /// Any error that occurs will be returned as a [`Box`].
    pub fn export_csv(self) -> Result<String, Box<dyn std::error::Error>> {
        let mut wtr = csv::WriterBuilder::new()
            .has_headers(false)
            .from_writer(vec![]);
        wtr.write_record(self.columns.into_iter().map(|c| c.ui_name))?; //Header row
        for row in self.rows {
            wtr.write_record(row.cells.into_iter().map(|c| c.data))?;
        }
        Ok(String::from_utf8(wtr.into_inner()?)?)
    }

    /// Decrypts all cells in this table that are marked as encrypted **in place**.
    ///
    /// # Arguments
    /// * `keypair`: The key pair that will be used to decrypt the cell's data
    ///
    /// *See also: [`TableCell`]*
    pub fn decrypt(&mut self, keypair: &crypt::KeyPair) {
        for row in &mut self.rows {
            for mut cell in &mut row.cells {
                if cell.encrypted {
                    if let Ok(plain) = keypair.decrypt_string_from_hex(&cell.data) {
                        cell.data = plain;
                    }
                }
            }
        }
    }
}
