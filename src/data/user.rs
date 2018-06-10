use rocket::outcome::IntoOutcome;
use rocket::request::{self, FromRequest, Request};

use diesel;
use diesel::prelude::*;

use data::db::Connection;
use data::schema::users;
use data::schema::users::dsl::*;

use oauth::twitter::TwitterUser;

use uuid;

#[derive(Debug)]
pub struct UserID {
	pub uuid: uuid::Uuid,
}

impl UserID {
	pub fn from_string(uuid_str: &str) -> Result<UserID, uuid::ParseError> {
		let uuid = uuid::Uuid::parse_str(uuid_str)?;
		Ok(UserID { uuid })
	}

	pub fn to_string(self: &UserID) -> String {
		format!("{}", self.uuid.hyphenated())
	}
}

#[derive(Debug)]
pub enum Error {
	UserNotFound,
}

impl<'a, 'r> FromRequest<'a, 'r> for UserID {
	type Error = Error;

	fn from_request(request: &'a Request<'r>) -> request::Outcome<UserID, Error> {
		request
			.cookies()
			.get_private("user_id")
			.and_then(|c_user_id| UserID::from_string(c_user_id.value()).ok())
			.and_then(|uuid| Some(uuid))
			.or_forward(())
	}
}

pub struct User<'a> {
	connection: &'a Connection,
	pub data: Data,
}

#[derive(Debug, Queryable, Identifiable)]
#[table_name = "users"]
pub struct Data {
	pub id: uuid::Uuid,
	pub twitter_name: Option<String>,
	pub twitter_screen_name: Option<String>,
}

impl<'a> User<'a> {
	pub fn get_by_id(connection: &'a Connection, p_user_id: &UserID) -> QueryResult<User<'a>> {
		let data = users
			.filter(id.eq(&p_user_id.uuid))
			.first::<Data>(&connection.pg_connection)?;

		Ok(User { connection, data })
	}

	pub fn get_by_twitter(
		connection: &'a Connection,
		twitter_user: &TwitterUser,
	) -> QueryResult<User<'a>> {
		let data = users
			.filter(twitter_screen_name.eq(&twitter_user.screen_name))
			.first::<Data>(&connection.pg_connection)?;

		Ok(User { connection, data })
	}

	pub fn create_from_twitter(
		connection: &'a Connection,
		twitter_user: &TwitterUser,
	) -> QueryResult<User<'a>> {
		let data = diesel::insert_into(users)
			.values(twitter_user)
			.get_result(&connection.pg_connection)?;

		Ok(User { connection, data })
	}

	pub fn get_documents(self: &User<'a>) -> QueryResult<Vec<super::document::Document<'a>>> {
		super::document::Document::get_by_user(self.connection, &self.data.id)
	}
}
