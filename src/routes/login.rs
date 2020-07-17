use config::SeriatimConfig;

use data::db::Connection;
use data::memory;
use data::memory::session::{Session, SessionID};
use data::user::User;

use oauth::{LoginMethod, OAuthResponse, OAuthSource};

use rocket;
use rocket::http::{Cookie, Cookies};
use rocket::outcome::IntoOutcome;
use rocket::request::{Form, FromRequest, Outcome, Request};
use rocket::{Route, State};

use routes::io::{send_success, SeriatimResult};

use std::net::IpAddr;
use std::rc::Rc;

const RETURN_URL_COOKIE: &'static str = "redirect_url";
const REDIRECT_USER_COOKIE: &'static str = "redirect_user_id";

#[derive(Serialize)]
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

#[derive(Serialize)]
struct ClientIP(IpAddr);

impl<'a, 'r> FromRequest<'a, 'r> for ClientIP {
    type Error = ();

    fn from_request(request: &'a Request<'r>) -> Outcome<Self, Self::Error> {
        request
            .client_ip()
            .and_then(|ip| Some(ClientIP(ip)))
            .or_forward(())
    }
}

fn get_callback(cfg: &SeriatimConfig, method: &LoginMethod, merge: bool) -> String {
    cfg.client.clone()
        + "login/"
        + &format!("{}", method).to_ascii_lowercase()
        + if merge { "/merge" } else { "/callback" }
}

#[get("/<login_method>/callback?<oauth_params..>")]
fn login_callback<'a>(
    client_ip: ClientIP,
    login_method: LoginMethod,
    oauth_params: Form<OAuthResponse>,
    redirect: ReturnURL,
    mut cookies: Cookies,
    con: Connection,
    redis: memory::redis::Connection,
    cfg: State<SeriatimConfig>,
) -> SeriatimResult {
    let callback = get_callback(&cfg, &login_method, false);

    let oauth_user = OAuthSource::create(&login_method, &cfg, callback)
        .get_oauth_token(&oauth_params)?
        .get_user()?;

    let db_user = match User::get_by_oauth_user(&con, &oauth_user) {
        Ok(u) => Ok(u),
        Err(_) => User::create_from_oauth_user(&con, &oauth_user),
    }?;

    let con = Rc::new(redis);

    let user_id = db_user.get_id();
    let session = Session::create(con.clone(), &user_id, &client_ip.0)?;

    if let Some(max) = cfg.max_user_sessions {
        Session::trim_oldest_sessions(con.clone(), &user_id, max)?;
    }
    let session_id_cookie = session.data.session_id.to_cookie();
    cookies.add_private(session_id_cookie);

    Ok(send_success(&ReturnURL { url: redirect.url }))
}

#[get("/<_login_method>/callback")]
fn login_denied<'a>(_login_method: LoginMethod, redirect: ReturnURL) -> SeriatimResult {
    Ok(send_success(&ReturnURL { url: redirect.url }))
}

#[get("/<login_method>/merge?<oauth_params..>")]
fn login_merge<'a>(
    // ip_addr: ClientIP,
    login_method: LoginMethod,
    oauth_params: Form<OAuthResponse>,
    redirect: ReturnURL,
    con: Connection,
    redis: memory::redis::Connection,
    mut cookies: Cookies,
    cfg: State<SeriatimConfig>,
) -> SeriatimResult {
    let redirect_session_id = match SessionID::from_named_cookie(&mut cookies, REDIRECT_USER_COOKIE)
    {
        Some(u) => Ok(u),
        None => Err(super::error::Error::NotLoggedIn),
    }?;

    let redirect_session = Session::get_by_id(Rc::new(redis), &redirect_session_id)?; //.check_ip(&ip_addr.0)?;

    let mut merge_into = User::get_by_id(&con, &redirect_session.data.user_id)?;

    let callback = get_callback(&cfg, &login_method, true);
    let oauth_user = OAuthSource::create(&login_method, &cfg, callback)
        .get_oauth_token(&oauth_params)?
        .get_user()?;

    let merge_from = match User::get_by_oauth_user(&con, &oauth_user) {
        Ok(u) => Ok(u),
        Err(_) => User::create_from_oauth_user(&con, &oauth_user),
    }?;

    merge_into.merge(&merge_from)?;

    Ok(send_success(&ReturnURL { url: redirect.url }))
}

#[get("/<_login_method>/merge")]
fn merge_denied<'a>(_login_method: LoginMethod, redirect: ReturnURL) -> SeriatimResult {
    Ok(send_success(&ReturnURL { url: redirect.url }))
}

#[get("/<login_method>?<url>&<merge>")]
fn login<'a>(
    login_method: LoginMethod,
    url: String,
    merge: Option<bool>,
    mut cookies: Cookies,
    cfg: State<SeriatimConfig>,
) -> SeriatimResult {
    let callback = get_callback(&cfg, &login_method, merge.unwrap_or(false));

    let mut auth = OAuthSource::create(&login_method, &cfg, callback);
    let redirect_url = auth.get_redirect_url()?;

    cookies.add(Cookie::new(RETURN_URL_COOKIE, url));

    if let Some(session_id) = SessionID::from_cookie(&mut cookies) {
        cookies.add_private(
            // have to set samesite policy so cookie is still around after the redirect
            Cookie::build(REDIRECT_USER_COOKIE, session_id.cookie_value())
                .http_only(false)
                .same_site(rocket::http::SameSite::Lax)
                .finish(),
        );
    }

    Ok(send_success(&ReturnURL { url: redirect_url }))
}

#[allow(unused_variables)]
#[get("/logout?<url>&<merge>")]
fn logout(url: String, merge: Option<bool>, mut cookies: Cookies) -> SeriatimResult {
    cookies
        .get_private(SessionID::cookie_name())
        .map(|c| cookies.remove_private(c));

    Ok(send_success(&ReturnURL { url }))
}

pub fn routes() -> Vec<Route> {
    routes![
        login,
        login_denied,
        login_merge,
        merge_denied,
        login_callback,
        logout
    ]
}
