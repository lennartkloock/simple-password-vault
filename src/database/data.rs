use crate::crypt;

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct Password {
    pub id: u64,
    pub name: String,
    pub password_hash: String,
    pub admin: bool,
    pub created: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct TableIndexEntry {
    pub id: u64,
    pub ui_name: String,
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct ColumnIndexEntry {
    pub id: u64,
    pub table_name: String,
    pub column_name: String,
    pub ui_name: String,
    pub required: bool,
    pub encrypted: bool,
}

#[derive(Default, Debug, serde::Serialize)]
pub struct VaultTable {
    pub id: u64,
    pub name: String,
    pub columns: Vec<ColumnIndexEntry>,
    pub rows: Vec<TableRow>,
}

#[derive(Default, Debug, serde::Serialize)]
pub struct TableRow {
    pub id: u64,
    pub cells: Vec<TableCell>,
}

#[derive(Default, Debug, serde::Serialize)]
pub struct TableCell {
    pub data: String,
    pub encrypted: bool,
}

impl VaultTable {
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
