use config::SeriatimConfig;

use data::db::Connection;
use data::user::User;
use data::user::UserID;

use oauth::{LoginMethod, OAuthResponse, OAuthSource};

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

fn get_callback(cfg: &SeriatimConfig, method: &LoginMethod) -> String {
	cfg.domain.clone() + "login/" + &format!("{}", method).to_ascii_lowercase() + "/callback"
}

#[get("/<login_method>/callback?<oauth_params>")]
fn login_callback<'a>(
	login_method: LoginMethod,
	oauth_params: OAuthResponse,
	redirect: ReturnURL,
	mut cookies: Cookies,
	con: Connection,
	cfg: State<SeriatimConfig>,
) -> Result<Response<'a>, Box<std::error::Error>> {
	let callback = get_callback(&cfg, &login_method);

	let oauth_user = OAuthSource::create(&login_method, &cfg, callback)
		.get_oauth_token(&oauth_params)?
		.get_user()?;

	let db_user = match User::get_by_oauth_user(&con, &oauth_user) {
		Ok(u) => Ok(u),
		Err(_) => User::create_from_oauth_user(&con, &oauth_user),
	}?;

	let user_id = db_user.get_id();
	let user_id_cookie = user_id.to_cookie();

	cookies.add_private(user_id_cookie);
	Ok(redirect_response(redirect.url))
}

#[get("/<_login_method>/callback")]
fn login_denied<'a>(_login_method: LoginMethod, redirect: ReturnURL) -> Response<'a> {
	redirect_response(redirect.url)
}

#[get("/<login_method>?<redirect>")]
fn login<'a>(
	login_method: LoginMethod,
	redirect: ReturnURL,
	mut cookies: Cookies,
	cfg: State<SeriatimConfig>,
) -> Response<'a> {
	let callback = get_callback(&cfg, &login_method);

	let mut auth = OAuthSource::create(&login_method, &cfg, callback);
	let oauth_url = auth.get_redirect_url();

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
	routes![login, login_denied, login_callback, logout]
}
