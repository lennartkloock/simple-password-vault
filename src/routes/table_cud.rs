#![allow(clippy::useless_conversion)] //Because of `#[field(...)]` in `AddTableData`

//! Contains all routes that create, update or delete (`CUD`) tables

use crate::database::VaultTable;
use crate::routes::{FlashContext, VaultResponse};
use crate::sessions::{TokenAuth, TokenAuthResult, WithCookie};
use crate::{crypt, templates, VaultConfig, VaultDb};
use rocket::{form, http, request};
use std::collections;

pub fn get_routes() -> Vec<rocket::Route> {
    rocket::routes![
        add,
        add_submit,
        add_data_submit,
        delete_data_submit,
        edit,
        delete_submit
    ]
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
    _auth: TokenAuth<WithCookie>,
    form: form::Form<AddTableData<'_>>,
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
        Ok(id) => VaultResponse::redirect_to(rocket::uri!(super::vault::vault_table_id(
            id,
            Option::<String>::None
        ))),
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
    _auth: TokenAuth<WithCookie>,
    form: form::Form<AddDataData<'_>>,
    keypair: &rocket::State<crypt::KeyPair>,
    database: &rocket::State<VaultDb>,
) -> VaultResponse<()> {
    if let Ok(index) = database.fetch_column_index_by_id(form.table_id).await {
        if index.is_empty() {
            VaultResponse::Err(http::Status::BadRequest)
        } else {
            let data: collections::HashMap<&str, String> = form
                .data
                .iter()
                .filter_map(|d| {
                    let entry = index.iter().find(|e| e.ui_name == *d.0)?;
                    let data = if entry.encrypted {
                        keypair.encrypt_string_to_hex(d.1).ok()?
                    } else {
                        d.1.to_string()
                    };
                    Some((entry.column_name.as_ref(), data))
                })
                .collect();
            if database
                .insert_vault_data(
                    form.table_id,
                    data.iter().map(|d| (*d.0, d.1.as_ref())).collect(),
                )
                .await
                .is_ok()
            {
                VaultResponse::redirect_to(rocket::uri!(super::vault::vault_table_id(
                    form.table_id,
                    Option::<String>::None
                )))
            } else {
                VaultResponse::Err(http::Status::InternalServerError)
            }
        }
    } else {
        VaultResponse::Err(http::Status::InternalServerError)
    }
}

#[derive(Debug, rocket::FromForm)]
struct DeleteDataData {
    table_id: u64,
    row_id: u64,
}

#[rocket::post("/delete-data", data = "<form>")]
async fn delete_data_submit(
    _auth: TokenAuth<WithCookie>,
    form: form::Form<DeleteDataData>,
    database: &rocket::State<VaultDb>,
) -> VaultResponse<()> {
    if database
        .delete_vault_row(form.table_id, form.row_id)
        .await
        .is_ok()
    {
        VaultResponse::redirect_to(rocket::uri!(super::vault::vault_table_id(
            form.table_id,
            Option::<String>::None
        )))
    } else {
        VaultResponse::Err(http::Status::InternalServerError)
    }
}

#[derive(serde::Serialize)]
struct EditTableContext {
    flash: FlashContext,
    table: VaultTable,
}

#[rocket::get("/edit?<id>")]
async fn edit(
    auth: TokenAuthResult<WithCookie>,
    id: u64,
    config: &rocket::State<VaultConfig>,
    flash: Option<request::FlashMessage<'_>>,
    database: &rocket::State<VaultDb>,
) -> VaultResponse<templates::Template> {
    if auth.is_err() {
        VaultResponse::redirect_to(rocket::uri!(super::authentication::login))
    } else {
        match database.fetch_table(id, &None).await {
            Ok(table) => table.map_or(VaultResponse::Err(http::Status::NotFound), |t| {
                VaultResponse::Ok(templates::Template::render(
                    "edit",
                    EditTableContext {
                        flash: FlashContext::default()
                            .with_config(config.inner())
                            .with_optional_flash(flash),
                        table: t,
                    },
                ))
            }),
            Err(_) => VaultResponse::Err(http::Status::InternalServerError),
        }
    }
}

#[derive(rocket::FromForm)]
struct DeleteData {
    table_id: u64,
}

#[rocket::post("/delete", data = "<form>")]
async fn delete_submit(
    _auth: TokenAuth<WithCookie>,
    form: form::Form<DeleteData>,
    database: &rocket::State<VaultDb>,
) -> VaultResponse<()> {
    database
        .delete_vault_table(form.table_id)
        .await
        .map(|_| VaultResponse::redirect_to(rocket::uri!(super::vault::vault)))
        .unwrap_or(VaultResponse::Err(http::Status::InternalServerError))
}
