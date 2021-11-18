use crate::routes::GeneralContext;
use crate::sessions::{TokenAuth, WithCookie, SESSION_TOKEN_COOKIE};
use crate::{templates, VaultConfig};
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
    config: &rocket::State<VaultConfig>,
    _auth: TokenAuth<WithCookie>,
) -> templates::Template {
    templates::Template::render("vault", GeneralContext::from(config.inner()))
}

#[rocket::get("/vault?<id>")]
async fn vault_table_id(
    config: &rocket::State<VaultConfig>,
    _auth: TokenAuth<WithCookie>,
    id: u32,
) -> templates::Template {
    templates::Template::render("table-not-found", GeneralContext::from(config.inner()));
    todo!()
}
