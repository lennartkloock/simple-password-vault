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
    database: &rocket::State<VaultDb>,
) -> VaultResponse<templates::Template> {
    if auth.is_err() {
        VaultResponse::redirect_to(rocket::uri!(super::authentication::login))
    } else {
        match database.fetch_table_index().await {
            Ok(index) => {
                if let Some(first) = index.first() {
                    VaultResponse::redirect_to(rocket::uri!(vault_table_id(first.id())))
                } else {
                    VaultResponse::Ok(templates::Template::render(
                        "no-tables",
                        GeneralContext::from(config.inner()),
                    ))
                }
            }
            Err(_) => VaultResponse::Err(http::Status::InternalServerError),
        }
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
            selected: false,
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

    fn with_tables(mut self, tables: Vec<TableContextTable>) -> Self {
        self.tables.extend(tables);
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
            Ok(t) => {
                let table_index = database.fetch_table_index().await; //Can't be done in map_or closure because of `await`, better solution?
                VaultResponse::Ok(t.map_or(
                    templates::Template::render(
                        "table-not-found",
                        GeneralContext::from(config.inner()),
                    ),
                    |table| {
                        // TODO: I'm sure the following code is pretty ugly but it works for now!
                        let selected_table_id = table.id;
                        let mut context_table: TableContextTable = table.into();
                        context_table.selected = true;
                        let mut tables = vec![context_table];
                        if let Ok(index) = table_index {
                            tables.extend(index.into_iter().filter_map(|e| {
                                (selected_table_id != e.id())
                                    .then(|| TableContextTable::from(VaultTable::from(e)))
                            }));
                        }
                        templates::Template::render(
                            "table",
                            TableContext::default()
                                .with_config(config)
                                .with_tables(tables),
                        )
                    },
                ))
            }
            Err(_) => VaultResponse::Err(http::Status::InternalServerError),
        }
    }
}
