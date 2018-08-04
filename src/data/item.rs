use diesel;
use diesel::prelude::*;

use data::db::Connection;
use data::document::DocumentID;
use data::schema::items;
use data::schema::items::dsl::*;

use serde::ser::{Serialize, SerializeStruct, Serializer};

use uuid;

#[derive(TaggedID)]
pub struct ItemID(uuid::Uuid);

pub struct Item<'a> {
	connection: &'a Connection,
	pub data: Data,
}

#[derive(Identifiable, AsChangeset, Queryable)]
#[table_name = "items"]
pub struct Data {
	id: uuid::Uuid,
	pub document_id: uuid::Uuid,
	pub parent_id: Option<uuid::Uuid>,
	pub item_text: String,
	pub child_order: i32,
	pub collapsed: bool,
}

impl<'a> Item<'a> {
	pub fn new(connection: &'a Connection, data: Data) -> Item<'a> {
		Item { connection, data }
	}

	pub fn get_id(&self) -> ItemID {
		ItemID::from_uuid(self.data.id.clone())
	}

	pub fn get_parent_id(&self) -> Option<ItemID> {
		self.data
			.parent_id
			.and_then(|p| Some(ItemID::from_uuid(p.clone())))
	}

	fn results_list(
		connection: &'a Connection,
		items_list: Vec<Data>,
	) -> QueryResult<Vec<Item<'a>>> {
		Ok(items_list
			.into_iter()
			.map(|data| Item { connection, data })
			.collect())
	}

	pub fn get_by_id(connection: &'a Connection, p_item_id: &ItemID) -> QueryResult<Item<'a>> {
		let p_uuid = **p_item_id;

		let data = items
			.filter(id.eq(&p_uuid))
			.first::<Data>(&connection.pg_connection)?;

		Ok(Item { connection, data })
	}

	pub fn get_by_document(
		connection: &'a Connection,
		p_document_id: &DocumentID,
	) -> QueryResult<Vec<Item<'a>>> {
		let p_doc_uuid = **p_document_id;

		let items_list = items
			.filter(document_id.eq(&p_doc_uuid))
			.load::<Data>(&connection.pg_connection)?;

		Self::results_list(connection, items_list)
	}

	pub fn remove_children(&mut self) -> QueryResult<usize> {
		let p_item_uuid = self.get_id();

		diesel::delete(items)
			.filter(parent_id.eq(&*p_item_uuid))
			.execute(&self.connection.pg_connection)
	}

	pub fn update_text(&mut self, update_text: &str) -> QueryResult<&mut Item<'a>> {
		let data = diesel::update(items)
			.filter(id.eq(self.data.id))
			.set(item_text.eq(update_text))
			.get_result(&self.connection.pg_connection)?;

		self.data = data;

		Ok(self)
	}
}

impl<'a> Serialize for Item<'a> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut serialized = serializer.serialize_struct("Item", 6)?;
		serialized.serialize_field("item_id", &self.get_id())?;

		serialized.serialize_field(
			"document_id",
			&self.data.document_id.hyphenated().to_string(),
		)?;

		serialized.serialize_field(
			"parent_id",
			&self
				.data
				.parent_id
				.and_then(|p| Some(p.hyphenated().to_string())),
		)?;

		serialized.serialize_field("text", &self.data.item_text)?;
		serialized.serialize_field("child_order", &self.data.child_order)?;
		serialized.serialize_field("collapsed", &self.data.collapsed)?;

		serialized.end()
	}
}
