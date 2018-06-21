#![feature(plugin, decl_macro, custom_derive, custom_attribute, associated_type_defaults)]
#![plugin(rocket_codegen)]

extern crate dotenv;
extern crate reqwest;
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;
#[macro_use]
extern crate dotenv_codegen;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate diesel;
extern crate uuid;
#[macro_use]
extern crate seriatim_codegen;
extern crate serde;
extern crate serde_json;

mod data;
mod oauth;
mod routes;

fn main() {
	let login_routes = routes::login::routes();
	let document_routes = routes::document::routes();
	let user_routes = routes::user::routes();

	rocket::ignite()
		.manage(data::db::init_pool())
		.mount("/document", document_routes)
		.mount("/login", login_routes)
		.mount("/user", user_routes)
		.launch();
}
