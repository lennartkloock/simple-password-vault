pub fn get_routes() -> Vec<rocket::Route> {
    rocket::routes![index]
}

#[rocket::get("/")]
fn index() -> &'static str {
    "Hello, world!"
}
