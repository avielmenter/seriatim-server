#![feature(proc_macro_hygiene, decl_macro, associated_type_defaults)]

extern crate base64;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_derive_enum;
extern crate dotenv;
extern crate hmacsha1;
extern crate r2d2_redis;
extern crate rand;
extern crate regex;
extern crate reqwest;
#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;
extern crate rocket_cors;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate url;
extern crate uuid;

#[macro_use]
extern crate seriatim_codegen;

mod config;
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

    let cors = rocket_cors::CorsOptions::default()
        .allow_credentials(true)
        .allowed_origins(rocket_cors::AllowedOrigins::some_exact(&[
            &cfg.allowed_origin
        ]))
        .to_cors()
        .unwrap();

    let db = data::db::init_pool(&cfg);
    let redis = data::memory::redis::init_pool(&cfg).unwrap();

    rocket::ignite()
        .manage(db)
        .manage(redis)
        .manage(cfg)
        .mount("/document", document_routes)
        .mount("/login", login_routes)
        .mount("/user", user_routes)
        .attach(cors)
        .launch();
}
