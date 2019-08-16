use diesel;

use rocket::http::ContentType;
use rocket::request::Request;
use rocket::response::{self, Responder, Response};

use std;
use std::ops::Deref;

use uuid::ParseError;

#[derive(Debug)]
pub enum Error {
	InsufficientPermissions,
	NotLoggedIn,
	TooFewLoginMethods,
	DatabaseError(Box<diesel::result::Error>),
	OtherError(Box<dyn std::error::Error>),
}

impl Error {
	pub fn code(&self) -> &'static str {
		match self {
			Error::InsufficientPermissions => "INSUFFICIENT_PERMISSIONS",
			Error::NotLoggedIn => "NOT_LOGGED_IN",
			Error::TooFewLoginMethods => "TOO_FEW_LOGIN_METHODS",
			Error::DatabaseError(e) => match e.deref() {
				diesel::result::Error::NotFound => "NOT_FOUND",
				_ => "DATABASE_ERROR",
			},
			_ => "OTHER_ERROR",
		}
	}
}

impl std::error::Error for Error {
	fn description(&self) -> &str {
		match self {
			Error::InsufficientPermissions => "Insufficient Permissions",
			Error::NotLoggedIn => "must be logged in to access this url",
			Error::TooFewLoginMethods => "must always have at least one way to log in",
			Error::DatabaseError(e) => e.description(),
			Error::OtherError(e) => e.description(),
		}
	}
}

impl<'r> Responder<'r> for Error {
	fn respond_to(self, _: &Request) -> response::Result<'r> {
		Response::build()
			.header(ContentType::JSON)
			.sized_body(std::io::Cursor::new(format!(
				"{{\"status\": \"error\", \"code\": \"{}\", \"error\": \"{}\" }}",
				self.code(),
				self
			)))
			.ok()
	}
}

impl std::fmt::Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Error::InsufficientPermissions => write!(f, "Insufficient Permissions - you do not have the permissions necessary to perform this action"),
			Error::NotLoggedIn => write!(f, "Not Logged In - you must be logged in to access this URL"),
			Error::TooFewLoginMethods => write!(f, "Too Few Login Methods - you can only remove a login method if you have at least one remaining way to log in"),
			Error::DatabaseError(e) => write!(f, "Database Error - {}", e),
			Error::OtherError(e) => write!(f, "Other Error - {}", e),
		}
	}
}

impl std::convert::From<diesel::result::Error> for Error {
	fn from(error: diesel::result::Error) -> Self {
		Error::DatabaseError(Box::new(error))
	}
}

pub trait NotSeriatimError {}

impl<E: std::error::Error + NotSeriatimError + 'static> std::convert::From<E> for Error {
	fn from(error: E) -> Self {
		Error::OtherError(Box::new(error))
	}
}

impl NotSeriatimError for ParseError {}
