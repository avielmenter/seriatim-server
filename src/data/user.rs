use data;
use data::db::Connection;
use data::schema::users;
use data::schema::users::dsl::*;

use diesel;
use diesel::prelude::*;

use oauth::google::GoogleUser;
use oauth::twitter::TwitterUser;
use oauth::OAuthUser;

use serde::ser::{Serialize, SerializeStruct, Serializer};

use uuid;

#[derive(TaggedID)]
pub struct UserID(uuid::Uuid);

pub struct User<'a> {
	connection: &'a Connection,
	pub data: Data,
}

#[derive(Insertable)]
#[table_name = "users"]
struct NewUser {
	pub display_name: String,
	pub google_id: Option<String>,
	pub twitter_screen_name: Option<String>,
	pub facebook_id: Option<String>,
}

#[derive(Debug, Queryable, Identifiable)]
#[table_name = "users"]
pub struct Data {
	id: uuid::Uuid,
	pub display_name: String,
	pub google_id: Option<String>,
	pub twitter_screen_name: Option<String>,
	pub facebook_id: Option<String>,
}

impl<'a> User<'a> {
	pub fn get_id(&self) -> UserID {
		UserID(self.data.id.clone())
	}

	pub fn get_by_id(connection: &'a Connection, p_user_id: &UserID) -> QueryResult<User<'a>> {
		let p_uuid = **p_user_id;

		let data = users
			.filter(id.eq(&p_uuid))
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

	pub fn get_by_google(
		connection: &'a Connection,
		google_user: &GoogleUser,
	) -> QueryResult<User<'a>> {
		let data = users
			.filter(google_id.eq(&google_user.id))
			.first::<Data>(&connection.pg_connection)?;

		Ok(User { connection, data })
	}

	pub fn get_by_oauth_user(
		connection: &'a Connection,
		oauth_user: &OAuthUser,
	) -> QueryResult<User<'a>> {
		match oauth_user {
			OAuthUser::Google(google_user) => User::get_by_google(connection, google_user),
			OAuthUser::Twitter(twitter_user) => User::get_by_twitter(connection, twitter_user),
		}
	}

	pub fn create_from_twitter(
		connection: &'a Connection,
		twitter_user: &TwitterUser,
	) -> QueryResult<User<'a>> {
		let data = diesel::insert_into(users)
			.values(NewUser {
				display_name: twitter_user.name.clone(),
				google_id: None,
				facebook_id: None,
				twitter_screen_name: Some(twitter_user.screen_name.clone()),
			})
			.get_result(&connection.pg_connection)?;

		Ok(User { connection, data })
	}

	pub fn create_from_google(
		connection: &'a Connection,
		google_user: &GoogleUser,
	) -> QueryResult<User<'a>> {
		let data = diesel::insert_into(users)
			.values(NewUser {
				display_name: google_user.name.clone(),
				google_id: Some(google_user.id.clone()),
				twitter_screen_name: None,
				facebook_id: None,
			})
			.get_result(&connection.pg_connection)?;

		Ok(User { connection, data })
	}

	pub fn create_from_oauth_user(
		connection: &'a Connection,
		oauth_user: &OAuthUser,
	) -> QueryResult<User<'a>> {
		match oauth_user {
			OAuthUser::Google(google_user) => User::create_from_google(connection, google_user),
			OAuthUser::Twitter(twitter_user) => User::create_from_twitter(connection, twitter_user),
		}
	}

	pub fn get_documents(self: &User<'a>) -> QueryResult<Vec<data::document::Document<'a>>> {
		data::document::Document::get_by_user(self.connection, &self.get_id())
	}

	pub fn create_document(&self) -> QueryResult<data::document::Document> {
		data::document::Document::create_for_user(&self.connection, &self.get_id())
	}
}

impl<'a> Serialize for User<'a> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut serialized = serializer.serialize_struct("User", 3)?;
		serialized.serialize_field("user_id", &self.get_id())?;
		serialized.serialize_field("display_name", &self.data.display_name)?;
		serialized.serialize_field("google_id", &self.data.google_id)?;
		serialized.serialize_field("twitter_screen_name", &self.data.twitter_screen_name)?;

		serialized.end()
	}
}
