// taken from from https://github.com/SergioBenitez/Rocket/issues/25#issuecomment-313895086
// thank you nicholasday!

use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::{ContentType, Header, Method};
use rocket::{Request, Response};

use std::env;
use std::io::Cursor;

pub struct CORS();

impl Fairing for CORS {
	fn info(&self) -> Info {
		Info {
			name: "Add CORS headers to requests",
			kind: Kind::Response,
		}
	}

	fn on_response(&self, request: &Request, response: &mut Response) {
		if request.method() == Method::Options || response.content_type() == Some(ContentType::JSON)
		{
			response.set_header(Header::new(
				"Access-Control-Allow-Origin",
				env::var("SERIATIM_ALLOWED_ORIGIN").unwrap(),
			));
			response.set_header(Header::new(
				"Access-Control-Allow-Methods",
				"POST, GET, DELETE, OPTIONS",
			));
			response.set_header(Header::new("Access-Control-Allow-Headers", "Content-Type"));
			fresponse.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
		}

		if request.method() == Method::Options {
			response.set_header(ContentType::Plain);
			response.set_sized_body(Cursor::new(""));
		}
	}
}
