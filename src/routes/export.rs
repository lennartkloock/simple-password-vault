//! Contains all routes of the export functionality

use crate::routes::VaultResponse;
use crate::sessions::{TokenAuth, WithCookie};
use crate::{crypt, VaultDb};
use rocket::http;

pub fn get_routes() -> Vec<rocket::Route> {
    rocket::routes![download]
}

#[rocket::get("/download/<id>")]
async fn download(
    id: u64,
    keypair: &rocket::State<crypt::KeyPair>,
    _auth: TokenAuth<WithCookie>,
    database: &rocket::State<VaultDb>,
) -> VaultResponse<(http::ContentType, String)> {
    match database.fetch_table(id).await.map(|table| {
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
