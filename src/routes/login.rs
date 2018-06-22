use data::db::Connection;
use data::user::User;

use oauth::twitter;
use oauth::twitter::Twitter;

use rocket;
use rocket::http::{Cookie, Cookies};
use rocket::outcome::IntoOutcome;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::response::{Flash, Response};
use rocket::Route;

use routes::io::redirect_response;

use std;

const RETURN_URL_COOKIE: &'static str = "redirect_url";

#[derive(FromForm)]
struct ReturnURL {
	url: String,
}

impl<'a, 'r> FromRequest<'a, 'r> for ReturnURL {
	type Error = ();

	fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
		request
			.cookies()
			.get(RETURN_URL_COOKIE)
			.and_then(|c| {
				println!("FOUND COOKIE");

				Some(ReturnURL {
					url: c.value().to_string(),
				})
			})
			.or_forward(())
	}
}

#[get("/twitter/callback?<oauth_params>")]
fn twitter_callback(
	oauth_params: twitter::TwitterOAuthQueryParams,
	redirect: ReturnURL,
	mut cookies: Cookies,
	con: Connection,
) -> Result<Flash<Response>, Box<std::error::Error>> {
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
	Ok(Flash::success(
		redirect_response(redirect.url),
		"Login successful!",
	))
}

#[get("/twitter?<redirect>")]
fn twitter_login(redirect: ReturnURL, mut cookies: Cookies) -> Response {
	let callback = dotenv!("SERIATIM_DOMAIN").to_string() + &"login/twitter/callback".to_string();
	let twitter_key = dotenv!("SERIATIM_TWITTER_KEY").to_string();
	let twitter_secret = dotenv!("SERIATIM_TWITTER_SECRET").to_string();

	let mut auth = Twitter::create(twitter_key, twitter_secret);
	let oauth_url = auth.get_redirect_url(callback);

	if let Ok(redirect_url) = oauth_url {
		println!("ADDING RETURN URL COOKIE: {}", redirect.url);
		cookies.add(Cookie::new(RETURN_URL_COOKIE, redirect.url));

		redirect_response(redirect_url)
	} else {
		rocket::response::Response::build()
			.status(rocket::http::Status::InternalServerError)
			.finalize()
	}
}

pub fn routes() -> Vec<Route> {
	routes![twitter_login, twitter_callback]
}
