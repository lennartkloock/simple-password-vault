//! Contains all routes for exporting and importing tables from CSV files

use crate::routes::VaultResponse;
use crate::sessions::{TokenAuth, WithCookie};
use crate::{crypt, VaultDb};
use rocket::{form, fs, http};
use std::path;

pub fn get_routes() -> Vec<rocket::Route> {
    rocket::routes![download, import_submit]
}

#[rocket::get("/download/<id>")]
async fn download(
    _auth: TokenAuth<WithCookie>,
    id: u64,
    keypair: &rocket::State<crypt::KeyPair>,
    database: &rocket::State<VaultDb>,
) -> VaultResponse<(http::ContentType, String)> {
    match database.fetch_table(id, &None).await.map(|table| {
        table.map(|mut t| {
            t.decrypt(keypair);
            t.export_csv()
        })
    }) {
        Ok(Some(Ok(csv))) => VaultResponse::Ok((http::ContentType::CSV, csv)),
        Ok(None) => VaultResponse::Err(http::Status::NotFound),
        _ => VaultResponse::Err(http::Status::InternalServerError),
    }
}

#[derive(rocket::FromForm)]
struct ImportData<'a> {
    table_id: u64,
    upload: fs::TempFile<'a>,
}

#[rocket::post("/import", data = "<form>")]
async fn import_submit(
    auth: TokenAuth<WithCookie>,
    mut form: form::Form<ImportData<'_>>,
    config: &rocket::Config,
    database: &rocket::State<VaultDb>,
    keypair: &rocket::State<crypt::KeyPair>,
) -> VaultResponse<()> {
    let mut path = config.temp_dir.clone();
    path.push(auth.token());
    if let (Ok(_), Some(p)) = (form.upload.persist_to(path).await, form.upload.path()) {
        match import(p, form.table_id, database, keypair).await {
            Ok(_) => VaultResponse::flash_success_redirect_to(
                rocket::uri!(super::table_cud::edit(form.table_id)),
                "The selected file was imported",
            ),
            Err(e) => match e {
                ImportError::CsvError(e) => response_from_csv_err(e.kind(), form.table_id),
                ImportError::ColumnMismatch => VaultResponse::flash_error_redirect_to(
                    rocket::uri!(super::table_cud::edit(form.table_id)),
                    "The selected file does not contain the same columns as the selected table",
                ),
                ImportError::DatabaseError => VaultResponse::Err(http::Status::InternalServerError),
                ImportError::TableNotFound => VaultResponse::Err(http::Status::NotFound),
            },
        }
    } else {
        VaultResponse::Err(http::Status::InternalServerError)
    }
}

enum ImportError {
    CsvError(csv::Error),
    ColumnMismatch,
    DatabaseError,
    TableNotFound,
}

// XXXX: This is pretty chunky, iterator magic though
async fn import(
    path: &path::Path,
    table_id: u64,
    database: &VaultDb,
    keypair: &crypt::KeyPair,
) -> Result<(), ImportError> {
    let r = csv::Reader::from_path(path)
        .map_err(ImportError::CsvError)?
        .into_records();
    let table = database
        .fetch_table(table_id, &None)
        .await
        .map_err(|_| ImportError::DatabaseError)?
        .ok_or(ImportError::TableNotFound)?;

    let mut records = vec![];
    for record in r {
        let record = record.map_err(ImportError::CsvError)?;
        records.push(
            (record.len() == table.columns.len())
                .then(|| record)
                .ok_or(ImportError::ColumnMismatch)?,
        );
    }

    for r in records.iter().map(|r| {
        table
            .columns
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                r.get(i).and_then(|ri| {
                    Some((
                        c.column_name.as_ref(),
                        if c.encrypted {
                            keypair.encrypt_string_to_hex(ri).ok()?
                        } else {
                            ri.to_string()
                        },
                    ))
                })
            })
            .collect::<Vec<(&str, String)>>()
    }) {
        database
            .insert_vault_data(table_id, r.iter().map(|d| (d.0, d.1.as_ref())).collect())
            .await
            .map_err(|_| ImportError::DatabaseError)?;
    }
    Ok(())
}

fn response_from_csv_err<T>(err_kind: &csv::ErrorKind, table_id: u64) -> VaultResponse<T> {
    match err_kind {
        csv::ErrorKind::Utf8 { .. }
        | csv::ErrorKind::UnequalLengths { .. }
        | csv::ErrorKind::Deserialize { .. } => VaultResponse::flash_error_redirect_to(
            rocket::uri!(super::table_cud::edit(table_id)),
            "The selected file does not contain valid csv data",
        ),
        _ => VaultResponse::Err(http::Status::InternalServerError),
    }
}
