use crate::routes::{FlashContext, VaultResponse};
use crate::sessions::{TokenAuth, TokenAuthResult, WithCookie};
use crate::{templates, VaultConfig, VaultDb};
use rocket::{form, http, request};

pub fn get_routes() -> Vec<rocket::Route> {
    rocket::routes![add, add_submit]
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
    extra: Vec<&'a str>,
}

#[rocket::post("/add", data = "<form>")]
async fn add_submit(
    form: form::Form<AddTableData<'_>>,
    _auth: TokenAuth<WithCookie>,
    database: &rocket::State<VaultDb>,
) -> VaultResponse<()> {
    match database.create_vault_table(&form.name, &form.extra).await {
        (Ok(_), Some(id)) => {
            VaultResponse::redirect_to(rocket::uri!(super::vault::vault_table_id(id as u32)))
        }
        (Err(sqlx::Error::Database(e)), _) => {
            VaultResponse::flash_error_redirect_to(rocket::uri!(add), e.message())
        }
        _ => VaultResponse::Err(http::Status::InternalServerError),
    }
}
