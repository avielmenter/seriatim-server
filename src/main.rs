#![feature(plugin, decl_macro, custom_derive)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate reqwest;
extern crate dotenv;
#[macro_use] extern crate dotenv_codegen;
#[macro_use] extern crate serde_derive;

mod oauth;
use oauth::twitter;
use oauth::twitter::Twitter;

#[get("/login/twitter/callback?<oauth_params>")]
fn twitter_callback(oauth_params: twitter::TwitterOAuthQueryParams) -> Result<String, Box<std::error::Error>> {
	let twitter_key = dotenv!("SERIATIM_TWITTER_KEY").to_string();
	let twitter_secret = dotenv!("SERIATIM_TWITTER_SECRET").to_string();

	let user = Twitter::create(twitter_key, twitter_secret)
		.verify_request_token(oauth_params.oauth_verifier, oauth_params.oauth_token)?
		.verify_credentials()?;

	Ok(format!("{:?}", user))
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

fn main() {
	rocket::ignite().mount("/", routes![twitter_login, twitter_callback]).launch();
}
