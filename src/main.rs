#![feature(plugin, decl_macro, custom_derive, custom_attribute, associated_type_defaults)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate diesel;
extern crate dotenv;
extern crate rand;
extern crate reqwest;
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate url;
extern crate uuid;

#[macro_use]
extern crate seriatim_codegen;

mod config;
mod cors;
mod data;
mod oauth;
mod routes;

fn main() {
	dotenv::dotenv().ok();

	let cfg = config::SeriatimConfig::init();

	println!("Seriatim Configuration:");
	println!("{}", cfg);

	let login_routes = routes::login::routes();
	let document_routes = routes::document::routes();
	let user_routes = routes::user::routes();

	rocket::ignite()
		.manage(data::db::init_pool(&cfg))
		.manage(cfg)
		.mount("/document", document_routes)
		.mount("/login", login_routes)
		.mount("/user", user_routes)
		.attach(cors::CORS())
		.launch();
}
