#![feature(plugin, decl_macro, custom_derive)]
#![plugin(rocket_codegen)]

extern crate dotenv;
extern crate reqwest;
extern crate rocket;
#[macro_use]
extern crate dotenv_codegen;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate diesel;
extern crate uuid;

mod oauth;
use oauth::twitter;
use oauth::twitter::Twitter;

mod data;
use data::db::Connection;
use data::user::*;

use rocket::http::{Cookie, Cookies};

#[get("/login/twitter/callback?<oauth_params>")]
fn twitter_callback(
	oauth_params: twitter::TwitterOAuthQueryParams,
	mut cookies: Cookies,
	con: Connection,
) -> Result<String, Box<std::error::Error>> {
	let twitter_key = dotenv!("SERIATIM_TWITTER_KEY").to_string();
	let twitter_secret = dotenv!("SERIATIM_TWITTER_SECRET").to_string();

	let twitter_user = Twitter::create(twitter_key, twitter_secret)
		.verify_request_token(oauth_params.oauth_verifier, oauth_params.oauth_token)?
		.verify_credentials()?;

	let db_user = match User::get_by_twitter(&con, &twitter_user) {
		Ok(u) => Ok(u),
		Err(_) => User::create_from_twitter(&con, &twitter_user),
	}?;

	cookies.add_private(Cookie::new("user_id", db_user.user_id.to_string()));
	Ok(format!("{:?}", db_user))
}

#[get("/login/twitter")]
fn twitter_login() -> rocket::response::Response<'static> {
	let callback = dotenv!("SERIATIM_DOMAIN").to_string() + &"login/twitter/callback".to_string();
	let twitter_key = dotenv!("SERIATIM_TWITTER_KEY").to_string();
	let twitter_secret = dotenv!("SERIATIM_TWITTER_SECRET").to_string();

	let mut auth = Twitter::create(twitter_key, twitter_secret);
	let oauth_url = auth.get_redirect_url(callback);

	if let Ok(redirect_url) = oauth_url {
		rocket::response::Response::build()
			.status(rocket::http::Status::Found)
			.raw_header("Location", redirect_url)
			.finalize()
	} else {
		rocket::response::Response::build()
			.status(rocket::http::Status::InternalServerError)
			.finalize()
	}
}

#[get("/document/<doc_id>")]
fn get_document(doc_id: String, user: UserID, con: Connection) -> String {
	let doc = data::document::Document::get_by_document_id(&con, &doc_id);

	match doc {
		Ok(d) => format!("Document found! ID: {}", d.document_id.hyphenated()),
		Err(e) => format!("Error finding document: {:?}", e),
	}
}

fn main() {
	rocket::ignite()
		.manage(data::db::init_pool())
		.mount("/", routes![twitter_login, twitter_callback, get_document])
		.launch();
}
