#![feature(plugin, decl_macro, custom_derive)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate reqwest;
extern crate dotenv;
#[macro_use] extern crate dotenv_codegen;

mod oauth;
use oauth::twitter;
use oauth::twitter::Twitter;

const ENV_DOMAIN: &'static str = "SERIATIM_DOMAIN";
const ENV_TWITTER_KEY: &'static str = "SERIATIM_TWITTER_KEY";
const ENV_TWITTER_SECRET: &'static str = "SERIATIM_TWITTER_SECRET";

#[get("/login/twitter/callback?<oauth_params>")]
fn twitter_callback(oauth_params: twitter::TwitterOAuthQueryParams) -> String {
	let twitter_key = dotenv!("SERIATIM_TWITTER_KEY").to_string();
	let twitter_secret = dotenv!("SERIATIM_TWITTER_SECRET").to_string();

	let mut auth = Twitter::create(twitter_key, twitter_secret);
	auth.verify_request_token(oauth_params.oauth_verifier, oauth_params.oauth_token);

	format!("{:?}", auth)
}

#[get("/login/twitter")]
fn twitter_login() -> rocket::response::Response<'static> {
	let callback = dotenv!("SERIATIM_DOMAIN").to_string() + &"login/twitter/callback".to_string();
	let twitter_key = dotenv!("SERIATIM_TWITTER_KEY").to_string();
	let twitter_secret = dotenv!("SERIATIM_TWITTER_SECRET").to_string();

	let mut auth = Twitter::create(twitter_key, twitter_secret);
	let oauth_url = auth.get_request_token(callback);

	rocket::response::Response::build()
		.status(rocket::http::Status::Found)
		.raw_header("Location", oauth_url)
		.finalize()
}

fn main() {
	rocket::ignite().mount("/", routes![twitter_login, twitter_callback]).launch();
}