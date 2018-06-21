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
