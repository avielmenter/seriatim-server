use oauth::http::OAuthRequest;
use reqwest;
use serde_json;
use std;
use std::collections::HashMap;

use config::SeriatimConfig;
use oauth::{FromOAuthResponse, OAuth, OAuthResponse};

#[allow(dead_code)]
#[derive(FromForm)]
pub struct FacebookOAuthResponse {
	code: String,
	state: String,
}

impl FromOAuthResponse for FacebookOAuthResponse {
	fn from_response(oauth_response: &OAuthResponse) -> Option<&Self> {
		match oauth_response {
			OAuthResponse::Facebook(facebook_response) => Some(facebook_response),
			_ => None,
		}
	}
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct FacebookTokenResponse {
	access_token: String,
	token_type: String,
	expires_in: i32,
}

#[derive(Deserialize)]
pub struct FacebookUser {
	pub id: String,
	pub name: String,
}

pub struct Facebook {
	app_id: String,
	app_secret: String,
	return_url: String,
	access_token: Option<String>,
}

impl OAuth for Facebook {
	type TResponse = FacebookOAuthResponse;
	type TUser = FacebookUser;

	fn get_redirect_url(&mut self) -> Result<String, reqwest::Error> {
		const OAUTH_REDIRECT_URL: &'static str = "https://www.facebook.com/v3.0/dialog/oauth?";

		Ok(OAUTH_REDIRECT_URL.to_string()
			+ "client_id="
			+ &self.app_id
			+ "&redirect_uri="
			+ &OAuthRequest::url_encode(&self.return_url)
			+ "&state={}")
	}

	fn get_oauth_token(
		&mut self,
		oauth_response: &Self::TResponse,
	) -> Result<&mut Self, Box<dyn std::error::Error>> {
		const OAUTH_TOKEN_URL: &'static str = "https://graph.facebook.com/v3.0/oauth/access_token?";

		let token_request_url = OAUTH_TOKEN_URL.to_string()
			+ "client_id="
			+ &self.app_id
			+ "&redirect_uri="
			+ &OAuthRequest::url_encode(&self.return_url)
			+ "&client_secret="
			+ &self.app_secret
			+ "&code="
			+ &oauth_response.code;

		let response =
			OAuthRequest::create(token_request_url, reqwest::Method::Get, HashMap::new())
				.get_response()?;

		let token: FacebookTokenResponse = serde_json::from_str(&response)?;
		self.access_token = Some(token.access_token);

		Ok(self)
	}

	fn get_user(&self) -> Result<FacebookUser, Box<dyn std::error::Error>> {
		const USER_INFO_URL: &'static str = "https://graph.facebook.com/me?";
		let info_url = USER_INFO_URL.to_string()
			+ "access_token="
			+ &self.access_token.clone().unwrap_or("".to_string())
			+ "&fields=id,name";

		println!("REQUESTING FACEBOOK USER INFO: {}", info_url);

		let response =
			OAuthRequest::create(info_url, reqwest::Method::Get, HashMap::new()).get_response()?;

		println!("FACEBOOK USER RESPONSE: {}", response);

		let u: FacebookUser = serde_json::from_str(&response)?;

		println!("PARSED USER INFO");
		Ok(u)
	}

	fn create(cfg: &SeriatimConfig, redirect_uri: String) -> Facebook {
		Facebook {
			app_id: cfg.fb_id.clone(),
			app_secret: cfg.fb_secret.clone(),
			return_url: redirect_uri,
			access_token: None,
		}
	}
}
