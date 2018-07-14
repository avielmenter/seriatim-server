pub mod google;
pub mod twitter;

use config::SeriatimConfig;
use reqwest;
use rocket::http::RawStr;
use rocket::request::{FormItems, FromForm, FromParam};
use std;
use std::fmt;
use std::io::ErrorKind;
use std::str::FromStr;

// LOGIN METHOD

pub enum LoginMethod {
	Twitter,
	Google,
}

impl LoginMethod {}

impl fmt::Display for LoginMethod {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			LoginMethod::Twitter => write!(f, "Twitter"),
			LoginMethod::Google => write!(f, "Google"),
		}
	}
}

impl FromStr for LoginMethod {
	type Err = std::io::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let sanitized = s.to_ascii_lowercase().trim().to_string();

		if sanitized == "twitter" {
			Ok(LoginMethod::Twitter)
		} else if sanitized == "google" {
			Ok(LoginMethod::Google)
		} else {
			Err(std::io::Error::new(
				ErrorKind::InvalidInput,
				"\"".to_string() + s + " is not a valid login method",
			))
		}
	}
}

impl<'r> FromParam<'r> for LoginMethod {
	type Error = std::io::Error;

	fn from_param(param: &'r RawStr) -> Result<Self, Self::Error> {
		let param_str: String = match param.url_decode() {
			Ok(s) => Ok(s),
			Err(_) => Err(std::io::Error::new(
				ErrorKind::InvalidInput,
				"Could not parse login method.",
			)),
		}?;

		LoginMethod::from_str(&param_str)
	}
}

// OAUTH RESPONSE

pub enum OAuthResponse {
	Google(google::GoogleOAuthResponse),
	Twitter(twitter::TwitterOAuthResponse),
}

impl<'f> FromForm<'f> for OAuthResponse {
	type Error = std::io::Error;

	fn from_form(items: &mut FormItems<'f>, strict: bool) -> Result<OAuthResponse, Self::Error> {
		println!("PARSING OAUTH RESPONSE");

		let items_str = items.inner_str();

		if let Ok(twitter_response) =
			twitter::TwitterOAuthResponse::from_form(&mut FormItems::from(items_str), strict)
		{
			Ok(OAuthResponse::Twitter(twitter_response))
		} else if let Ok(google_response) =
			google::GoogleOAuthResponse::from_form(&mut FormItems::from(items_str), strict)
		{
			Ok(OAuthResponse::Google(google_response))
		} else {
			Err(std::io::Error::new(
				ErrorKind::InvalidInput,
				"Could not parse OAuth response",
			))
		}
	}
}

pub trait FromOAuthResponse: Sized {
	fn from_response(response: &OAuthResponse) -> Option<&Self>;
}

// OAUTH USER

pub enum OAuthUser {
	Google(google::GoogleUser),
	Twitter(twitter::TwitterUser),
}

impl From<google::GoogleUser> for OAuthUser {
	fn from(user: google::GoogleUser) -> Self {
		OAuthUser::Google(user)
	}
}

impl From<twitter::TwitterUser> for OAuthUser {
	fn from(user: twitter::TwitterUser) -> Self {
		OAuthUser::Twitter(user)
	}
}

// OAUTH TRAIT

pub trait OAuth {
	type TResponse;
	type TUser;

	fn create(cfg: &SeriatimConfig, redirect_url: String) -> Self;

	fn get_redirect_url(&mut self) -> Result<String, reqwest::Error>;

	fn get_oauth_token(
		&mut self,
		oauth_response: &Self::TResponse,
	) -> Result<&mut Self, Box<std::error::Error>>;

	fn get_user(&self) -> Result<Self::TUser, Box<std::error::Error>>;
}

// OAUTH SOURCE

pub enum OAuthSource {
	Google(google::Google),
	Twitter(twitter::Twitter),
}

impl OAuthSource {
	fn parse_response<T: FromOAuthResponse>(
		response: &OAuthResponse,
	) -> Result<&T, std::io::Error> {
		match T::from_response(&response) {
			Some(parsed) => Ok(parsed),
			None => Err(std::io::Error::new(
				ErrorKind::InvalidInput,
				"The OAuth response type did not match the specified login method.",
			)),
		}
	}

	pub fn create(method: &LoginMethod, cfg: &SeriatimConfig, callback: String) -> OAuthSource {
		match method {
			LoginMethod::Google => OAuthSource::Google(google::Google::create(cfg, callback)),
			LoginMethod::Twitter => OAuthSource::Twitter(twitter::Twitter::create(cfg, callback)),
		}
	}

	pub fn get_redirect_url(&mut self) -> Result<String, reqwest::Error> {
		match self {
			OAuthSource::Google(g) => g.get_redirect_url(),
			OAuthSource::Twitter(t) => t.get_redirect_url(),
		}
	}

	pub fn get_oauth_token(
		&mut self,
		oauth_response: &OAuthResponse,
	) -> Result<&mut Self, Box<std::error::Error>> {
		match self {
			OAuthSource::Google(g) => {
				let r =
					Self::parse_response::<<google::Google as OAuth>::TResponse>(&oauth_response)?;
				g.get_oauth_token(&r)?;
			}
			OAuthSource::Twitter(t) => {
				let r = Self::parse_response::<<twitter::Twitter as OAuth>::TResponse>(
					&oauth_response,
				)?;
				t.get_oauth_token(&r)?;
			}
		};

		Ok(self)
	}

	pub fn get_user(&self) -> Result<OAuthUser, Box<std::error::Error>> {
		match self {
			OAuthSource::Google(g) => Ok(OAuthUser::from(g.get_user()?)),
			OAuthSource::Twitter(t) => Ok(OAuthUser::from(t.get_user()?)),
		}
	}
}
