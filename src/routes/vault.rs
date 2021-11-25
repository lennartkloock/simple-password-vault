//! Contains all routes that can be used to read tables

use crate::database::VaultTable;
use crate::routes::{GeneralContext, VaultResponse};
use crate::sessions::{TokenAuthResult, WithCookie, SESSION_TOKEN_COOKIE};
use crate::{templates, VaultConfig, VaultDb};
use rocket::{http, response};

pub fn get_routes() -> Vec<rocket::Route> {
    rocket::routes![index, vault, vault_table_id]
}

#[rocket::get("/")]
async fn index(cookies: &http::CookieJar<'_>) -> response::Redirect {
    if cookies.get(SESSION_TOKEN_COOKIE).is_some() {
        response::Redirect::to(rocket::uri!(vault))
    } else {
        response::Redirect::to(rocket::uri!(super::authentication::login))
    }
}

#[rocket::get("/vault")]
async fn vault(
    auth: TokenAuthResult<WithCookie>,
    config: &rocket::State<VaultConfig>,
) -> VaultResponse<templates::Template> {
    if auth.is_err() {
        VaultResponse::redirect_to(rocket::uri!(super::authentication::login))
    } else {
        VaultResponse::Ok(templates::Template::render(
            "no-tables",
            GeneralContext::from(config.inner()),
        ))
    }
}

#[derive(serde::Serialize)]
struct TableContextTable {
    id: u64,
    name: String,
    selected: bool,
    columns: Vec<String>,
    data: Vec<Vec<String>>,
}

impl From<VaultTable> for TableContextTable {
    fn from(table: VaultTable) -> Self {
        let mut columns = vec!["Number".to_string(), "Password".to_string()];
        columns.extend(table.extra_columns);
        Self {
            id: table.id,
            name: table.name,
            selected: true,
            columns,
            data: table.data,
        }
    }
}

#[derive(Default, serde::Serialize)]
struct TableContext {
    general: GeneralContext,
    tables: Vec<TableContextTable>,
}

impl TableContext {
    fn with_general_context(mut self, general: GeneralContext) -> Self {
        self.general = general;
        self
    }

    fn with_config(self, config: &VaultConfig) -> Self {
        self.with_general_context(GeneralContext::from(config))
    }

    fn with_vault_table(mut self, table: VaultTable) -> Self {
        self.tables.push(table.into());
        self
    }
}

#[rocket::get("/vault?<id>")]
async fn vault_table_id(
    id: u64,
    auth: TokenAuthResult<WithCookie>,
    config: &rocket::State<VaultConfig>,
    database: &rocket::State<VaultDb>,
) -> VaultResponse<templates::Template> {
    if auth.is_err() {
        VaultResponse::redirect_to(rocket::uri!(super::authentication::login))
    } else {
        match database.fetch_table(id).await {
            Ok(t) => VaultResponse::Ok(t.map_or(
                templates::Template::render(
                    "table-not-found",
                    GeneralContext::from(config.inner()),
                ),
                |table| {
                    templates::Template::render(
                        "table",
                        TableContext::default()
                            .with_config(config)
                            .with_vault_table(table),
                    )
                },
            )),
            Err(_) => VaultResponse::Err(http::Status::InternalServerError),
        }
    }
}
