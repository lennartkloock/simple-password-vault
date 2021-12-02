//! Contains all routes that create, update or delete (`CUD`) tables

use crate::routes::{FlashContext, VaultResponse};
use crate::sessions::{TokenAuth, TokenAuthResult, WithCookie};
use crate::{templates, VaultConfig, VaultDb};
use rocket::{form, http, request};
use std::collections;

pub fn get_routes() -> Vec<rocket::Route> {
    rocket::routes![add, add_submit, add_data_submit]
}

#[rocket::get("/add")]
async fn add(
    auth: TokenAuthResult<WithCookie>,
    config: &rocket::State<VaultConfig>,
    flash: Option<request::FlashMessage<'_>>,
) -> VaultResponse<templates::Template> {
    if auth.is_err() {
        VaultResponse::redirect_to(rocket::uri!(super::authentication::login))
    } else {
        let context = FlashContext::default()
            .with_config(config)
            .with_optional_flash(flash);
        VaultResponse::Ok(templates::Template::render("add-table", context))
    }
}

#[derive(Debug, rocket::FromForm)]
struct AddTableData<'a> {
    name: &'a str,
    #[field(default = "Key")]
    key_column_name: &'a str,
    #[field(default = "Password")]
    password_column_name: &'a str,
    extra: Vec<&'a str>,
}

#[rocket::post("/add", data = "<form>")]
async fn add_submit(
    form: form::Form<AddTableData<'_>>,
    _auth: TokenAuth<WithCookie>,
    database: &rocket::State<VaultDb>,
) -> VaultResponse<()> {
    let key_ui_name = if form.key_column_name.is_empty() {
        "Key"
    } else {
        form.key_column_name
    };
    let password_ui_name = if form.password_column_name.is_empty() {
        "Password"
    } else {
        form.password_column_name
    };

    match database
        .create_vault_table(form.name, key_ui_name, password_ui_name, &form.extra)
        .await
    {
        Ok(id) => VaultResponse::redirect_to(rocket::uri!(super::vault::vault_table_id(id))),
        Err(sqlx::Error::Database(e)) => {
            VaultResponse::flash_error_redirect_to(rocket::uri!(add), e.message())
        }
        _ => VaultResponse::Err(http::Status::InternalServerError),
    }
}

#[derive(Debug, rocket::FromForm)]
struct AddDataData<'a> {
    table_id: u64,
    data: collections::HashMap<&'a str, &'a str>,
}

#[rocket::post("/add-data", data = "<form>")]
async fn add_data_submit(
    form: form::Form<AddDataData<'_>>,
    _auth: TokenAuth<WithCookie>,
    database: &rocket::State<VaultDb>,
) -> VaultResponse<()> {
    if let Ok(index) = database.fetch_column_index_by_id(form.table_id).await {
        if index.is_empty() {
            VaultResponse::Err(http::Status::BadRequest)
        } else {
            let data: collections::HashMap<&str, &str> = form
                .data
                .iter()
                .filter_map(|d| {
                    let entry = index.iter().find(|e| e.ui_name == *d.0)?;
                    Some((entry.column_name.as_ref(), *d.1))
                })
                .collect();
            if database
                .insert_vault_data(form.table_id, data)
                .await
                .is_ok()
            {
                VaultResponse::redirect_to(rocket::uri!(super::vault::vault_table_id(
                    form.table_id
                )))
            } else {
                VaultResponse::Err(http::Status::InternalServerError)
            }
        }
    } else {
        VaultResponse::Err(http::Status::InternalServerError)
    }
}
