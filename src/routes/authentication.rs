use crate::routes::GeneralContext;
use crate::{templates, VaultConfig, VaultDb};
use rocket::{form, http, response};

pub fn get_routes() -> Vec<rocket::Route> {
    rocket::routes![login, new_admin_password, new_admin_password_form,]
}

type ServerResponse<T> = Result<T, http::Status>;

#[rocket::get("/login")]
async fn login(
    config: &rocket::State<VaultConfig>,
    database: &rocket::State<VaultDb>,
) -> ServerResponse<Result<templates::Template, response::Redirect>> {
    if database
        .fetch_all_password(true)
        .await
        .map_err(|_| http::Status::InternalServerError)?
        .is_empty()
    {
        Ok(Err(response::Redirect::to(rocket::uri!(
            new_admin_password
        ))))
    } else {
        Ok(Ok(templates::Template::render(
            "login",
            GeneralContext::from(config.inner()),
        )))
    }
}

#[rocket::get("/new-admin-password")]
async fn new_admin_password(
    config: &rocket::State<VaultConfig>,
    database: &rocket::State<VaultDb>,
) -> ServerResponse<Result<templates::Template, response::Redirect>> {
    if database
        .fetch_all_password(true)
        .await
        .map_err(|_| http::Status::InternalServerError)?
        .is_empty()
    {
        Ok(Ok(templates::Template::render(
            "new-admin-password",
            GeneralContext::from(config.inner()),
        )))
    } else {
        Ok(Err(response::Redirect::to(rocket::uri!(login))))
    }
}

#[derive(rocket::FromForm)]
struct NewAdminPasswordData<'a> {
    password: &'a str,
    #[field(name = "password-confirm", validate = eq(self.password))]
    _password_confirm: &'a str,
}

#[rocket::post("/new-admin-password", data = "<form>")]
async fn new_admin_password_form(
    database: &rocket::State<VaultDb>,
    form: form::Form<form::Contextual<'_, NewAdminPasswordData<'_>>>,
) -> Result<response::Redirect, ServerResponse<String>> {
    if database
        .fetch_all_password(true)
        .await
        .map_err(|_| Err(http::Status::InternalServerError))?
        .is_empty()
    {
        if let Some(ref data) = form.value {
            database
                .insert_password(data.password, true)
                .await
                .map(|_| response::Redirect::to(rocket::uri!(login)))
                .map_err(|_| Err(http::Status::InternalServerError))
        } else {
            Err(Ok(form
                .context
                .field_errors("password-confirm")
                .fold(String::new(), |i, e| format!("{:?}\n{}", e, i))))
        }
    } else {
        Ok(response::Redirect::to(rocket::uri!(login)))
    }
}
