use base64;
use hmacsha1;
use rand;
use reqwest;
use serde_json;
use std;
use std::collections::HashMap;
use url;

use super::{FromOAuthResponse, OAuth, OAuthResponse};
use config::SeriatimConfig;

const NONCE_LENGTH: u32 = 32;
const OAUTH_VERSION: &'static str = "1.0";
const OAUTH_SIGNATURE_METHOD: &'static str = "HMAC-SHA1";

#[derive(FromForm)]
pub struct TwitterOAuthResponse {
	pub oauth_token: String,
	pub oauth_verifier: String,
}

impl FromOAuthResponse for TwitterOAuthResponse {
	fn from_response(oauth_response: &OAuthResponse) -> Option<&Self> {
		match oauth_response {
			OAuthResponse::Twitter(twitter_response) => Some(twitter_response),
			_ => None,
		}
	}
}

pub struct Twitter {
	oauth_consumer_key: String,
	oauth_consumer_secret: String,
	redirect_url: String,
	oauth_token_secret: Option<String>,
	oauth_token: Option<String>,
}

#[derive(Deserialize)]
pub struct TwitterUser {
	pub name: String,
	pub screen_name: String,
}

struct TwitterRequest<'a> {
	twitter: &'a Twitter,
	url: String,
	method: reqwest::Method,
	oauth_timestamp: u64,
	oauth_nonce: String,
	oauth_version: String,
	oauth_signature_method: String,
	header_params: HashMap<String, String>,
	body_params: HashMap<String, String>,
}

impl Twitter {
	fn get_timestamp() -> u64 {
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap_or(std::time::Duration::new(0, 0)) // just return 0 if no timestamp can be found
			.as_secs()
	}

	fn get_nonce() -> String {
		(0..NONCE_LENGTH)
			.map(|_| format!("{:x}", rand::random::<u8>()))
			.fold(String::new(), |acc, x| acc + &x)
	}

	fn generate_request(
		&self,
		method: reqwest::Method,
		url: String,
		header_params: HashMap<String, String>,
		body_params: HashMap<String, String>,
	) -> TwitterRequest {
		TwitterRequest {
			twitter: self,
			url,
			method,
			oauth_timestamp: Twitter::get_timestamp(),
			oauth_nonce: Twitter::get_nonce(),
			oauth_signature_method: OAUTH_SIGNATURE_METHOD.to_string(),
			oauth_version: OAUTH_VERSION.to_string(),
			header_params,
			body_params,
		}
	}

	fn get_token_from_response(&mut self, response: String) {
		let vars = url::form_urlencoded::parse(response.as_bytes());
		for (key, val) in vars {
			if key == "oauth_token" {
				self.oauth_token = Some(val.to_string());
			} else if key == "oauth_token_secret" {
				self.oauth_token_secret = Some(val.to_string());
			}
		}
	}
}

impl OAuth for Twitter {
	type TResponse = TwitterOAuthResponse;
	type TUser = TwitterUser;

	fn get_redirect_url(&mut self) -> Result<String, reqwest::Error> {
		const REQUEST_TOKEN_URL: &'static str = "https://api.twitter.com/oauth/request_token";
		const OAUTH_TOKEN_URL: &'static str =
			"https://api.twitter.com/oauth/authenticate?oauth_token=";

		let mut params: HashMap<String, String> = HashMap::new();
		params.insert("oauth_callback".to_string(), self.redirect_url.clone());

		self.oauth_token = None;
		self.oauth_token_secret = None;

		let response =
			self.generate_request(
				reqwest::Method::Post,
				REQUEST_TOKEN_URL.to_string(),
				params,
				HashMap::new(),
			).get_response()?;

		self.get_token_from_response(response);

		Ok(
			OAUTH_TOKEN_URL.to_string() + &(if let Some(ref token) = self.oauth_token {
				token.clone()
			} else {
				"".to_string()
			}),
		)
	}

	fn get_oauth_token(
		&mut self,
		oauth_response: &TwitterOAuthResponse,
	) -> Result<&mut Twitter, Box<std::error::Error>> {
		const VERIFY_TOKEN_URL: &'static str = "https://api.twitter.com/oauth/access_token";

		let mut params: HashMap<String, String> = HashMap::new();
		params.insert(
			"oauth_verifier".to_string(),
			oauth_response.oauth_verifier.clone(),
		);

		self.oauth_token = Some(oauth_response.oauth_token.clone());
		self.oauth_token_secret = None;

		let response =
			self.generate_request(
				reqwest::Method::Post,
				VERIFY_TOKEN_URL.to_string(),
				params,
				HashMap::new(),
			).get_response()?;

		self.get_token_from_response(response);
		Ok(self)
	}

	fn get_user(&self) -> Result<TwitterUser, Box<std::error::Error>> {
		const VERIFY_CREDENTIALS_URL: &'static str =
			"https://api.twitter.com/1.1/account/verify_credentials.json";

		let response: String =
			self.generate_request(
				reqwest::Method::Get,
				VERIFY_CREDENTIALS_URL.to_string(),
				HashMap::new(),
				HashMap::new(),
			).get_response()?;

		let u: TwitterUser = serde_json::from_str(&response)?;

		Ok(u)
	}

	fn create(cfg: &SeriatimConfig, redirect_url: String) -> Twitter {
		Twitter {
			oauth_consumer_key: cfg.twitter_key.clone(),
			oauth_consumer_secret: cfg.twitter_secret.clone(),
			redirect_url,
			oauth_token_secret: None,
			oauth_token: None,
		}
	}
}

impl<'a> TwitterRequest<'a> {
	fn get_http_method_string(method: &reqwest::Method) -> String {
		match method {
			reqwest::Method::Options => "OPTIONS".to_string(),
			reqwest::Method::Get => "GET".to_string(),
			reqwest::Method::Post => "POST".to_string(),
			reqwest::Method::Put => "PUT".to_string(),
			reqwest::Method::Delete => "DELETE".to_string(),
			reqwest::Method::Head => "HEAD".to_string(),
			reqwest::Method::Trace => "TRACE".to_string(),
			reqwest::Method::Connect => "CONNECT".to_string(),
			reqwest::Method::Patch => "PATCH".to_string(),
			reqwest::Method::Extension(ext) => ext.clone(),
		}
	}

	fn url_encode(val: &str) -> String {
		url::form_urlencoded::byte_serialize(val.as_bytes()).collect()
	}

	fn get_params_vector(params: &HashMap<String, String>) -> Vec<(String, String)> {
		return params
			.iter()
			.map(|(key, val)| {
				(
					TwitterRequest::url_encode(key),
					TwitterRequest::url_encode(val),
				)
			})
			.collect();
	}

	fn get_signature(&self, request_params: &HashMap<String, String>) -> String {
		let mut sorted_params = TwitterRequest::get_params_vector(&request_params);
		sorted_params.append(&mut TwitterRequest::get_params_vector(&self.body_params));

		sorted_params.sort_by(|(key_a, _), (key_b, _)| key_a.cmp(key_b));

		let encoded_params = sorted_params.iter().fold(String::new(), |acc, (key, val)| {
			let ampersand = if acc == "" { "" } else { "&" };
			acc + ampersand + key + "=" + val
		});

		let unhashed = TwitterRequest::get_http_method_string(&self.method)
			+ "&"
			+ &TwitterRequest::url_encode(&self.url)
			+ "&"
			+ &TwitterRequest::url_encode(&encoded_params);

		let mut secret = "".to_string();
		if let Some(ref oauth_token_secret) = self.twitter.oauth_token_secret {
			secret = oauth_token_secret.clone();
		}

		let signing_key = self.twitter.oauth_consumer_secret.clone() + "&" + &secret;
		let hashed = hmacsha1::hmac_sha1(signing_key.as_bytes(), unhashed.as_bytes());

		base64::encode(&hashed)
	}

	fn build_auth_header(&self) -> reqwest::header::Headers {
		let mut request_params = self.header_params.clone();

		request_params.insert(
			"oauth_consumer_key".to_string(),
			self.twitter.oauth_consumer_key.clone(),
		);

		request_params.insert(
			"oauth_timestamp".to_string(),
			self.oauth_timestamp.to_string(),
		);

		request_params.insert("oauth_nonce".to_string(), self.oauth_nonce.clone());
		request_params.insert("oauth_version".to_string(), self.oauth_version.clone());

		request_params.insert(
			"oauth_signature_method".to_string(),
			self.oauth_signature_method.clone(),
		);

		if let Some(ref oauth_token) = self.twitter.oauth_token {
			request_params.insert("oauth_token".to_string(), oauth_token.clone());
		}

		let oauth_signature: String;
		{
			oauth_signature = self.get_signature(&request_params);
		}
		request_params.insert("oauth_signature".to_string(), oauth_signature);

		let mut params_sorted: Vec<(String, String)> = request_params.into_iter().collect();
		params_sorted.sort_by(|(key_a, _), (key_b, _)| key_a.cmp(key_b));

		let header_string = "OAuth ".to_string()
			+ &params_sorted
				.iter()
				.map(|(key, val)| key.clone() + "=\"" + &TwitterRequest::url_encode(val) + "\"")
				.fold(String::from(""), |acc, x| {
					let comma = if acc == "" { "" } else { "," };
					acc + comma + &x
				});

		let mut headers = reqwest::header::Headers::new();
		headers.set(reqwest::header::Authorization(header_string.clone()));

		headers
	}

	fn get_body_string(&self) -> String {
		let mut sorted_params = TwitterRequest::get_params_vector(&self.body_params);
		sorted_params.sort_by(|(key_a, _), (key_b, _)| key_a.cmp(key_b));

		sorted_params
			.iter()
			.map(|(key, val)| key.clone() + "=" + &TwitterRequest::url_encode(val))
			.fold(String::from(""), |acc, x| {
				let ampersand = if acc == "" { "" } else { "," };
				acc + ampersand + &x
			})
	}

	fn get_response(&self) -> Result<String, reqwest::Error> {
		let http_client = reqwest::Client::new();

		http_client
			.request(self.method.clone(), &self.url)
			.headers(self.build_auth_header())
			.body(self.get_body_string())
			.send()?
			.text()
	}
}
