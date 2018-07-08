use config::SeriatimConfig;

use data::db::Connection;
use data::user::User;
use data::user::UserID;

use oauth::twitter;
use oauth::twitter::Twitter;

use rocket;
use rocket::http::{Cookie, Cookies};
use rocket::outcome::IntoOutcome;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::response::Response;
use rocket::Route;
use rocket::State;

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
				Some(ReturnURL {
					url: c.value().to_string(),
				})
			})
			.or_forward(())
	}
}

#[get("/twitter/callback?<oauth_params>")]
fn twitter_callback<'a>(
	oauth_params: twitter::TwitterOAuthQueryParams,
	redirect: ReturnURL,
	mut cookies: Cookies,
	con: Connection,
	cfg: State<SeriatimConfig>,
) -> Result<Response<'a>, Box<std::error::Error>> {
	let twitter_user = Twitter::create(cfg.twitter_key.clone(), cfg.twitter_secret.clone())
		.verify_request_token(oauth_params.oauth_verifier, oauth_params.oauth_token)?
		.verify_credentials()?;

	let db_user = match User::get_by_twitter(&con, &twitter_user) {
		Ok(u) => Ok(u),
		Err(_) => User::create_from_twitter(&con, &twitter_user),
	}?;

	let user_id = db_user.get_id();

	let user_id_cookie = user_id.to_cookie();

	cookies.add_private(user_id_cookie);
	Ok(redirect_response(redirect.url))
}

#[get("/twitter?<redirect>")]
fn twitter_login<'a>(
	redirect: ReturnURL,
	mut cookies: Cookies,
	cfg: State<SeriatimConfig>,
) -> Response<'a> {
	let callback = cfg.domain.clone() + &"login/twitter/callback".to_string();

	let mut auth = Twitter::create(cfg.twitter_key.clone(), cfg.twitter_secret.clone());
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

#[get("/logout?<redirect>")]
fn logout(redirect: ReturnURL, mut cookies: Cookies) -> Response {
	cookies
		.get_private(UserID::cookie_name())
		.map(|c| cookies.remove_private(c));

	redirect_response(redirect.url)
}

pub fn routes() -> Vec<Route> {
	routes![twitter_login, twitter_callback, logout]
}
