use data;
use data::db::Connection;
use data::schema::users;
use data::schema::users::dsl::*;

use diesel;
use diesel::prelude::*;

use oauth::twitter::TwitterUser;

use serde::ser::{Serialize, SerializeStruct, Serializer};

use uuid;

#[derive(TaggedID)]
pub struct UserID(uuid::Uuid);

pub struct User<'a> {
	connection: &'a Connection,
	pub data: Data,
}

#[derive(Debug, Queryable, Identifiable)]
#[table_name = "users"]
pub struct Data {
	id: uuid::Uuid,
	pub twitter_name: Option<String>,
	pub twitter_screen_name: Option<String>,
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

	pub fn create_from_twitter(
		connection: &'a Connection,
		twitter_user: &TwitterUser,
	) -> QueryResult<User<'a>> {
		let data = diesel::insert_into(users)
			.values(twitter_user)
			.get_result(&connection.pg_connection)?;

		Ok(User { connection, data })
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
		let mut serialized = serializer.serialize_struct("Document", 3)?;
		serialized.serialize_field("user_id", &self.get_id())?;
		serialized.serialize_field("twitter_name", &self.data.twitter_name)?;
		serialized.serialize_field("twitter_screen_name", &self.data.twitter_screen_name)?;

		serialized.end()
	}
}
