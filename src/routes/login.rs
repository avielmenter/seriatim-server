use data::db::Connection;
use data::user::User;

use oauth::twitter;
use oauth::twitter::Twitter;

use rocket;
use rocket::http::{Cookie, Cookies};
use rocket::outcome::IntoOutcome;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::response::Response;
use rocket::Route;

use routes::io::redirect_response;

use std;
use std::env;

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
) -> Result<Response, Box<std::error::Error>> {
	let twitter_key = env::var("SERIATIM_TWITTER_KEY").unwrap();
	let twitter_secret = env::var("SERIATIM_TWITTER_SECRET").unwrap();

	let twitter_user = Twitter::create(twitter_key, twitter_secret)
		.verify_request_token(oauth_params.oauth_verifier, oauth_params.oauth_token)?
		.verify_credentials()?;

	let db_user = match User::get_by_twitter(&con, &twitter_user) {
		Ok(u) => Ok(u),
		Err(_) => User::create_from_twitter(&con, &twitter_user),
	}?;

	let user_id = db_user.get_id();

	let mut user_id_cookie = user_id.to_cookie();
	/*user_id_cookie.set_domain(env::var("SERIATIM_SESSION_DOMAIN").unwrap());
	user_id_cookie.set_path("/"); */
	user_id_cookie.make_permanent();

	cookies.add_private(user_id_cookie);
	Ok(redirect_response(redirect.url))
}

#[get("/twitter?<redirect>")]
fn twitter_login(redirect: ReturnURL, mut cookies: Cookies) -> Response {
	let callback = env::var("SERIATIM_DOMAIN").unwrap() + &"login/twitter/callback".to_string();
	let twitter_key = env::var("SERIATIM_TWITTER_KEY").unwrap();
	let twitter_secret = env::var("SERIATIM_TWITTER_SECRET").unwrap();

	let mut auth = Twitter::create(twitter_key, twitter_secret);
	let oauth_url = auth.get_redirect_url(callback);

	if let Ok(redirect_url) = oauth_url {
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
