use data::db::Connection;
use data::user::User;

use oauth::twitter;
use oauth::twitter::Twitter;

use rocket;
use rocket::http::Cookies;
use rocket::Route;

use rocket_contrib::JsonValue;

use std;

#[get("/twitter/callback?<oauth_params>")]
fn twitter_callback(
	oauth_params: twitter::TwitterOAuthQueryParams,
	mut cookies: Cookies,
	con: Connection,
) -> Result<JsonValue, Box<std::error::Error>> {
	let twitter_key = dotenv!("SERIATIM_TWITTER_KEY").to_string();
	let twitter_secret = dotenv!("SERIATIM_TWITTER_SECRET").to_string();

	let twitter_user = Twitter::create(twitter_key, twitter_secret)
		.verify_request_token(oauth_params.oauth_verifier, oauth_params.oauth_token)?
		.verify_credentials()?;

	let db_user = match User::get_by_twitter(&con, &twitter_user) {
		Ok(u) => Ok(u),
		Err(_) => User::create_from_twitter(&con, &twitter_user),
	}?;

	let user_id = db_user.get_id();

	cookies.add_private(user_id.to_cookie());

	Ok(json!({
		"id": user_id,
		"twitter_name": twitter_user.name,
		"twitter_screen_name": twitter_user.screen_name
	}))
}

#[get("/twitter")]
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

pub fn routes() -> Vec<Route> {
	routes![twitter_login, twitter_callback]
}
