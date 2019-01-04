use oauth::http::OAuthRequest;
use reqwest;
use serde_json;
use std;
use std::collections::HashMap;

use config::SeriatimConfig;
use oauth::{FromOAuthResponse, OAuth, OAuthResponse};

const SCOPES: &'static str = "https://www.googleapis.com/auth/userinfo.profile";

#[derive(FromForm)]
pub struct GoogleOAuthResponse {
	code: String,
	scope: String
}

impl FromOAuthResponse for GoogleOAuthResponse {
	fn from_response(oauth_response: &OAuthResponse) -> Option<&Self> {
		match oauth_response {
			OAuthResponse::Google(google_response) => Some(google_response),
			_ => None,
		}
	}
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct GoogleTokenResponse {
	access_token: String,
	refresh_token: Option<String>,
	expires_in: i32,
	token_type: String,
}

pub struct Google {
	client_id: String,
	client_secret: String,
	api_key: String,
	redirect_uri: String,
	access_token: Option<String>,
	refresh_token: Option<String>,
}

#[derive(Deserialize)]
struct GoogleUserSource {
	id: String,
}

#[derive(Deserialize)]
struct GoogleUserMetadata {
	source: GoogleUserSource,
}

#[allow(non_snake_case)]
#[derive(Deserialize)]
struct GoogleUserName {
	metadata: GoogleUserMetadata,
	displayName: String,
}

#[derive(Deserialize)]
struct GoogleUserRaw {
	names: Vec<GoogleUserName>,
}

pub struct GoogleUser {
	pub name: String,
	pub id: String,
}

impl Google {}

impl OAuth for Google {
	type TResponse = GoogleOAuthResponse;
	type TUser = GoogleUser;

	fn get_redirect_url(&mut self) -> Result<String, reqwest::Error> {
		const OAUTH_REDIRECT_URL: &'static str =
			"https://accounts.google.com/o/oauth2/v2/auth?response_type=code&access_type=offline";

		Ok(OAUTH_REDIRECT_URL.to_string()
			+ "&scope="
			+ &SCOPES
			+ "&client_id="
			+ &self.client_id
			+ "&redirect_uri="
			+ &OAuthRequest::url_encode(&self.redirect_uri))
	}

	fn get_oauth_token(
		&mut self,
		oauth_response: &GoogleOAuthResponse,
	) -> Result<&mut Google, Box<std::error::Error>> {
		const OAUTH_TOKEN_URL: &'static str = "https://www.googleapis.com/oauth2/v4/token";

		let mut body_params: HashMap<String, String> = HashMap::new();
		body_params.insert("code".to_string(), oauth_response.code.clone());
		body_params.insert("client_id".to_string(), self.client_id.clone());
		body_params.insert("client_secret".to_string(), self.client_secret.clone());
		body_params.insert("redirect_uri".to_string(), self.redirect_uri.clone());
		body_params.insert("grant_type".to_string(), "authorization_code".to_string());

		let response = OAuthRequest::create(
			OAUTH_TOKEN_URL.to_string(),
			reqwest::Method::Post,
			body_params,
		).get_response()?;

		let token: GoogleTokenResponse = serde_json::from_str(&response)?;

		self.access_token = Some(token.access_token);
		self.refresh_token = token.refresh_token;

		Ok(self)
	}

	fn get_user(&self) -> Result<GoogleUser, Box<std::error::Error>> {
		const GET_USER_URL: &'static str =
			"https://people.googleapis.com/v1/people/me?requestMask.includeField=person.names";

		let access_token = match self.access_token {
			Some(ref token) => token.clone(),
			None => "".to_string(),
		};

		let url =
			GET_USER_URL.to_string() + "&key=" + &self.api_key + "&access_token=" + &access_token;

		let response =
			OAuthRequest::create(url.clone(), reqwest::Method::Get, HashMap::new()).get_response()?;

		let u: GoogleUserRaw = serde_json::from_str(&response)?;

		if u.names.len() == 0 {
			Err(Box::new(std::io::Error::new(
				std::io::ErrorKind::InvalidData,
				"Could not parse Google profile information",
			)))
		} else {
			Ok(GoogleUser {
				name: u.names[0].displayName.clone(),
				id: u.names[0].metadata.source.id.clone(),
			})
		}
	}

	fn create(cfg: &SeriatimConfig, redirect_uri: String) -> Google {
		Google {
			client_id: cfg.google_id.clone(),
			client_secret: cfg.google_secret.clone(),
			api_key: cfg.google_api_key.clone(),
			redirect_uri,
			access_token: None,
			refresh_token: None,
		}
	}
}
