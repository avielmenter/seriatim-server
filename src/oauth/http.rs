use reqwest;
use std::collections::HashMap;
use url;

pub struct OAuthRequest {
	url: String,
	method: reqwest::Method,
	body_params: HashMap<String, String>,
}

impl OAuthRequest {
	pub fn url_encode(val: &str) -> String {
		url::form_urlencoded::byte_serialize(val.as_bytes()).collect()
	}

	pub fn get_response(&self) -> Result<String, reqwest::Error> {
		let http_client = reqwest::Client::new();

		http_client
			.request(self.method.clone(), &self.url)
			.form(&self.body_params)
			.send()?
			.text()
	}

	pub fn create(
		url: String,
		method: reqwest::Method,
		body_params: HashMap<String, String>,
	) -> OAuthRequest {
		OAuthRequest {
			url,
			method,
			body_params,
		}
	}
}
