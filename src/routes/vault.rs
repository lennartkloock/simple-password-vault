//! Contains all routes that can be used to read tables

use crate::database::{TableIndexEntry, VaultTable};
use crate::routes::{GeneralContext, VaultResponse};
use crate::sessions::{SafeSessionManager, TokenAuthResult, WithCookie, SESSION_TOKEN_COOKIE};
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
                    VaultResponse::redirect_to(rocket::uri!(vault_table_id(first.id)))
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

#[derive(Default, serde::Serialize)]
struct TableContext {
    general: GeneralContext,
    selected_table: VaultTable,
    tables: Vec<TableIndexEntry>,
}

impl TableContext {
    fn with_general_context(mut self, general: GeneralContext) -> Self {
        self.general = general;
        self
    }

    fn with_selected_table(mut self, table: VaultTable) -> Self {
        self.selected_table = table;
        self
    }

    fn with_tables(mut self, tables: Vec<TableIndexEntry>) -> Self {
        self.tables.extend(tables);
        self
    }
}

#[rocket::get("/vault?<id>")]
async fn vault_table_id(
    id: u64,
    auth: TokenAuthResult<WithCookie>,
    config: &rocket::State<VaultConfig>,
    session_manager: &rocket::State<SafeSessionManager>,
    database: &rocket::State<VaultDb>,
) -> VaultResponse<templates::Template> {
    if let Ok(token) = auth {
        match database.fetch_table(id).await {
            Ok(t) => {
                let table_index = database.fetch_table_index().await; //XXXX: Can't be done in map_or closure because of `await`, better solution?
                let admin = session_manager
                    .lock()
                    .await
                    .is_admin_session(token.token())
                    .unwrap_or(false); //XXXX: Same as above
                VaultResponse::Ok(t.map_or(
                    templates::Template::render(
                        "table-not-found",
                        GeneralContext::from(config.inner()),
                    ),
                    |table| {
                        let mut context = TableContext::default();
                        if let Ok(mut other_tables) = table_index {
                            other_tables.retain(|e| e.id != table.id); //Remove the selected table from the list, otherwise it would appear twice in the drop-down
                            context = context.with_tables(other_tables);
                        }
                        templates::Template::render(
                            "table",
                            context
                                .with_general_context(GeneralContext {
                                    admin,
                                    ..GeneralContext::from(config.inner())
                                })
                                .with_selected_table(table),
                        )
                    },
                ))
            }
            Err(_) => VaultResponse::Err(http::Status::InternalServerError),
        }
    } else {
        VaultResponse::redirect_to(rocket::uri!(super::authentication::login))
    }
}
