pub mod facebook;
pub mod google;
pub mod http;
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
	Facebook,
}

impl LoginMethod {}

impl fmt::Display for LoginMethod {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			LoginMethod::Twitter => write!(f, "Twitter"),
			LoginMethod::Google => write!(f, "Google"),
			LoginMethod::Facebook => write!(f, "Facebook"),
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
		} else if sanitized == "facebook" {
			Ok(LoginMethod::Facebook)
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
	Facebook(facebook::FacebookOAuthResponse),
}

impl<'f> FromForm<'f> for OAuthResponse {
	type Error = std::io::Error;

	fn from_form(items: &mut FormItems<'f>, strict: bool) -> Result<OAuthResponse, Self::Error> {
		let items_str = items
			.map(|item| item.key_value())
			.map(|(key, value)| key.to_string() + &String::from("=") + &value.to_string())
			.fold(String::from(""), |acc, i| acc + &i + &String::from("&"));

		println!("OAuth Params String: {}", items_str);

		if let Ok(twitter_response) =
			twitter::TwitterOAuthResponse::from_form(&mut FormItems::from(&items_str[..]), strict)
		{
			Ok(OAuthResponse::Twitter(twitter_response))
		} else if let Ok(google_response) =
			google::GoogleOAuthResponse::from_form(&mut FormItems::from(&items_str[..]), strict)
		{
			Ok(OAuthResponse::Google(google_response))
		} else if let Ok(facebook_response) =
			facebook::FacebookOAuthResponse::from_form(&mut FormItems::from(&items_str[..]), strict)
		{
			Ok(OAuthResponse::Facebook(facebook_response))
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
	Facebook(facebook::FacebookUser),
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

impl From<facebook::FacebookUser> for OAuthUser {
	fn from(user: facebook::FacebookUser) -> Self {
		OAuthUser::Facebook(user)
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
	) -> Result<&mut Self, Box<dyn std::error::Error>>;

	fn get_user(&self) -> Result<Self::TUser, Box<dyn std::error::Error>>;
}

// OAUTH SOURCE

pub enum OAuthSource {
	Google(google::Google),
	Twitter(twitter::Twitter),
	Facebook(facebook::Facebook),
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
			LoginMethod::Facebook => {
				OAuthSource::Facebook(facebook::Facebook::create(cfg, callback))
			}
		}
	}

	pub fn get_redirect_url(&mut self) -> Result<String, reqwest::Error> {
		match self {
			OAuthSource::Google(g) => g.get_redirect_url(),
			OAuthSource::Twitter(t) => t.get_redirect_url(),
			OAuthSource::Facebook(f) => f.get_redirect_url(),
		}
	}

	pub fn get_oauth_token(
		&mut self,
		oauth_response: &OAuthResponse,
	) -> Result<&mut Self, Box<dyn std::error::Error>> {
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
			OAuthSource::Facebook(f) => {
				let r = Self::parse_response::<<facebook::Facebook as OAuth>::TResponse>(
					&oauth_response,
				)?;
				f.get_oauth_token(&r)?;
			}
		};

		Ok(self)
	}

	pub fn get_user(&self) -> Result<OAuthUser, Box<dyn std::error::Error>> {
		match self {
			OAuthSource::Google(g) => Ok(OAuthUser::from(g.get_user()?)),
			OAuthSource::Twitter(t) => Ok(OAuthUser::from(t.get_user()?)),
			OAuthSource::Facebook(f) => Ok(OAuthUser::from(f.get_user()?)),
		}
	}
}
