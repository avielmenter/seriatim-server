use rocket::outcome::IntoOutcome;
use rocket::request::{self, FromRequest, Request};

use diesel;
use diesel::prelude::*;

use super::db::Connection;
use super::schema::users::dsl::*;

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

#[derive(Debug, Queryable)]
pub struct User {
	pub user_id: uuid::Uuid,
	pub twitter_name: Option<String>,
	pub twitter_screen_name: Option<String>,
}

impl User {
	pub fn get_by_user_id(con: &Connection, p_user_id: &UserID) -> QueryResult<User> {
		users
			.filter(user_id.eq(&p_user_id.uuid))
			.first::<User>(&con.pg_connection)
	}

	pub fn get_by_twitter(con: &Connection, twitter_user: &TwitterUser) -> QueryResult<User> {
		users
			.filter(twitter_screen_name.eq(&twitter_user.screen_name))
			.first::<User>(&con.pg_connection)
	}

	pub fn create_from_twitter(con: &Connection, twitter_user: &TwitterUser) -> QueryResult<User> {
		diesel::insert_into(users)
			.values(twitter_user)
			.get_result(&con.pg_connection)
	}
}
