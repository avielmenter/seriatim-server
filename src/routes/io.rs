use rocket;
use rocket_contrib::JsonValue;
use routes::error::Error;

use serde::ser::Serialize;

use std::env;
use std::time::SystemTime;

pub type SeriatimResult = Result<JsonValue, Error>;

pub fn send_success<T: Serialize>(data: &T) -> JsonValue {
	json!({
		"status": "success",
		"timestamp": SystemTime::now(),
		"data": data
	})
}

pub fn send_with_permissions<T: Serialize, P: Serialize>(data: &T, permissions: &P) -> JsonValue {
	json!({
		"status": "success",
		"timestamp": SystemTime::now(),
		"permissions": permissions,
		"data": data
	})
}

pub fn redirect_response(url: String) -> rocket::response::Response<'static> {
	rocket::response::Response::build()
		.status(rocket::http::Status::Found)//SeeOther)
		.raw_header("Location", url)
		.finalize()
}

pub fn cors_response<'a>() -> rocket::response::Response<'a> {
	rocket::response::Response::build()
		.raw_header(
			"Access-Control-Allow-Origin",
			env::var("SERIATIM_ALLOWED_ORIGIN").unwrap(),
		)
		.raw_header("Access-Control-Allow-Methods", "GET, POST, DELETE, OPTIONS")
		.raw_header("Access-Control-Allow-Headers", "Content-Type")
		.raw_header("Access-Control-Allow-Credentials", "true")
		.finalize()
}
