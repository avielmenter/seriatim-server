use rocket;
use rocket_contrib::JsonValue;
use routes::error::Error;
use serde::ser::Serialize;

pub type SeriatimResult = Result<JsonValue, Error>;

pub fn send_success<T: Serialize>(data: &T) -> JsonValue {
	json!({
		"status": "success",
		"data": data
	})
}

pub fn redirect_response(url: String) -> rocket::response::Response<'static> {
	rocket::response::Response::build()
		.status(rocket::http::Status::SeeOther)
		.raw_header("Location", url)
		.finalize()
}
