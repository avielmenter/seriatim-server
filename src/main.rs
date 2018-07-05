#![feature(plugin, decl_macro, custom_derive, custom_attribute, associated_type_defaults)]
#![plugin(rocket_codegen)]

extern crate dotenv;
extern crate reqwest;
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate diesel;
extern crate serde;
extern crate serde_json;
extern crate url;
extern crate uuid;
#[macro_use]
extern crate seriatim_codegen;

mod cors;
mod data;
mod oauth;
mod routes;

use std::env;

fn main() {
	dotenv::dotenv().ok();

	let login_routes = routes::login::routes();
	let document_routes = routes::document::routes();
	let user_routes = routes::user::routes();

	println!("DOMAIN: {}", env::var("SERIATIM_DOMAIN").unwrap());

	println!(
		"ALLOWED ORIGIN: {}",
		env::var("SERIATIM_ALLOWED_ORIGIN").unwrap()
	);

	println!(
		"SESSION DOMAIN: {}",
		env::var("SERIATIM_SESSION_DOMAIN").unwrap()
	);

	rocket::ignite()
		.manage(data::db::init_pool())
		.mount("/document", document_routes)
		.mount("/login", login_routes)
		.mount("/user", user_routes)
		.attach(cors::CORS())
		.launch();
}
