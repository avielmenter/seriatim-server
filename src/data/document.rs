use diesel;
use diesel::prelude::*;

use data::db::Connection;
use data::schema::documents;
use data::schema::documents::dsl::*;

use uuid;

use std::time::SystemTime;

pub struct Document<'a> {
	connection: &'a Connection,
	pub data: Data,
}

#[derive(Debug, Queryable, Identifiable)]
#[table_name = "documents"]
pub struct Data {
	pub id: uuid::Uuid,
	pub user_id: uuid::Uuid,
	pub root_item_id: Option<uuid::Uuid>,
	pub created_at: SystemTime,
	pub modified_at: Option<SystemTime>,
}

#[derive(Insertable)]
#[table_name = "documents"]
struct NewDocument<'a> {
	pub user_id: &'a uuid::Uuid,
}

impl<'a> Document<'a> {
	pub fn get_by_id(
		connection: &'a Connection,
		p_document_id: &uuid::Uuid,
	) -> QueryResult<Document<'a>> {
		let data = documents
			.filter(id.eq(&p_document_id))
			.first::<Data>(&connection.pg_connection)?;

		Ok(Document { connection, data })
	}

	pub fn create_for_user(
		connection: &'a Connection,
		p_user_id: &uuid::Uuid,
	) -> QueryResult<Document<'a>> {
		let data = diesel::insert_into(documents)
			.values(NewDocument {
				user_id: &p_user_id,
			})
			.get_result(&connection.pg_connection)?;

		Ok(Document { connection, data })
	}

	pub fn get_by_user(
		connection: &'a Connection,
		p_user_id: &uuid::Uuid,
	) -> QueryResult<Vec<Document<'a>>> {
		let docs = documents
			.filter(user_id.eq(&p_user_id))
			.load::<Data>(&connection.pg_connection)?;

		Ok(docs
			.into_iter()
			.map(|data| Document { connection, data })
			.collect())
	}

	pub fn can_be_viewed_by(self: &Document<'a>, p_user_id: &uuid::Uuid) -> QueryResult<bool> {
		Ok(self.data.id.eq(&p_user_id))
	}

	pub fn can_be_edited_by(self: &Document<'a>, p_user_id: &uuid::Uuid) -> QueryResult<bool> {
		Ok(self.data.id.eq(&p_user_id))
	}

	pub fn get_items(self: &Document<'a>) -> QueryResult<Vec<super::item::Item<'a>>> {
		super::item::Item::get_by_document(self.connection, &self.data.id)
	}
}
