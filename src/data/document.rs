use diesel;
use diesel::prelude::*;

use data;
use data::db::Connection;
use data::schema::documents;
use data::schema::documents::dsl::*;

use uuid;

use std::time::SystemTime;

#[derive(Debug, Queryable)]
pub struct Document {
	pub document_id: uuid::Uuid,
	pub user_id: uuid::Uuid,
	pub root_item_id: Option<uuid::Uuid>,
	pub created_at: SystemTime,
	pub modified_at: Option<SystemTime>,
}

#[derive(Insertable)]
#[table_name = "documents"]
struct NewDocument<'a> {
	#[column_name = "user_id"]
	pub user_id: &'a uuid::Uuid,
}

impl Document {
	pub fn get_by_document_id(con: &Connection, p_document_id: &str) -> QueryResult<Document> {
		let document_uuid = uuid::Uuid::parse_str(p_document_id).ok();
		if let None = document_uuid {
			return Err(diesel::result::Error::NotFound);
		}

		documents
			.filter(document_id.eq(document_uuid.unwrap()))
			.first::<Document>(&con.pg_connection)
	}

	pub fn create_for_user(con: &Connection, user: &data::user::User) -> QueryResult<Document> {
		diesel::insert_into(documents)
			.values(NewDocument {
				user_id: &user.user_id,
			})
			.get_result(&con.pg_connection)
	}
}
