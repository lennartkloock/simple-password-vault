use crate::routes::VaultResponse;
use crate::sessions::{TokenAuth, WithCookie};
use crate::VaultDb;
use rocket::http;

pub fn get_routes() -> Vec<rocket::Route> {
    rocket::routes![download]
}

#[rocket::get("/download/<id>")]
async fn download(
    id: u64,
    _auth: TokenAuth<WithCookie>,
    database: &rocket::State<VaultDb>,
) -> VaultResponse<(http::ContentType, String)> {
    match database
        .fetch_table(id)
        .await
        .map(|table| table.map(|t| t.export_csv()))
    {
        Ok(Some(Ok(csv))) => VaultResponse::Ok((http::ContentType::CSV, csv)),
        Ok(None) => VaultResponse::Err(http::Status::NotFound),
        _ => VaultResponse::Err(http::Status::InternalServerError),
    }
}
