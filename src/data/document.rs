use diesel;
use diesel::prelude::*;

use data;
use data::db::Connection;
use data::item::{Item, ItemID};
use data::schema::documents;
use data::schema::documents::dsl::*;
use data::schema::items;
use data::schema::items::dsl::*;
use data::user::UserID;

use uuid;

use serde::ser::{Serialize, SerializeStruct, Serializer};

use std::collections::HashMap;
use std::time::SystemTime;

#[derive(TaggedID)]
pub struct DocumentID(uuid::Uuid);

pub struct Document<'a> {
	connection: &'a Connection,
	pub data: Data,
}

pub struct DocumentWithItems<'a> {
	document: &'a Document<'a>,
	items_hashmap: HashMap<ItemID, Item<'a>>,
}

#[derive(Debug, Queryable, Identifiable)]
#[table_name = "documents"]
pub struct Data {
	id: uuid::Uuid,
	pub user_id: uuid::Uuid,
	root_item_id: Option<uuid::Uuid>,
	pub created_at: SystemTime,
	pub modified_at: Option<SystemTime>,
}

#[derive(Insertable)]
#[table_name = "documents"]
struct NewDocument<'a> {
	pub user_id: &'a uuid::Uuid,
}

#[derive(Insertable)]
#[table_name = "items"]
struct NewItem<'a> {
	document_id: uuid::Uuid,
	parent_id: Option<uuid::Uuid>,
	item_text: &'a str,
	child_order: i32,
	collapsed: bool,
}

impl<'a> Document<'a> {
	pub fn get_id(&self) -> DocumentID {
		DocumentID::from_uuid(self.data.id.clone())
	}

	pub fn get_by_id(
		connection: &'a Connection,
		p_document_id: &DocumentID,
	) -> QueryResult<Document<'a>> {
		let p_doc_uuid: uuid::Uuid = **p_document_id;

		let data = documents
			.filter(data::schema::documents::dsl::id.eq(&p_doc_uuid))
			.first::<Data>(&connection.pg_connection)?;

		Ok(Document { connection, data })
	}

	pub fn add_item(&mut self, p_parent_id: Option<ItemID>, p_order: i32) -> QueryResult<Item> {
		let insert_item = NewItem {
			document_id: *self.get_id(),
			parent_id: match p_parent_id {
				Some(pid) => Some(*pid),
				None => None,
			},
			item_text: "",
			child_order: p_order,
			collapsed: false,
		};

		let data = diesel::insert_into(items)
			.values(insert_item)
			.get_result(&self.connection.pg_connection)?;

		Ok(Item::new(self.connection, data))
	}

	pub fn create_for_user(
		connection: &'a Connection,
		p_user_id: &UserID,
	) -> QueryResult<Document<'a>> {
		let p_user_uuid = **p_user_id;

		let data: Data = diesel::insert_into(documents)
			.values(NewDocument {
				user_id: &p_user_uuid,
			})
			.get_result(&connection.pg_connection)?;

		let mut doc = Document { connection, data };
		let doc_id = doc.data.id.clone();
		let root_item = doc.add_item(None, 0)?;

		let rooted_data = diesel::update(documents)
			.filter(data::schema::documents::dsl::id.eq(&doc_id))
			.set(root_item_id.eq(&*root_item.get_id()))
			.get_result(&connection.pg_connection)?;

		Ok(Document {
			connection,
			data: rooted_data,
		})
	}

	pub fn get_by_user(
		connection: &'a Connection,
		p_user_id: &UserID,
	) -> QueryResult<Vec<Document<'a>>> {
		let p_user_uuid = **p_user_id;

		let docs = documents
			.filter(user_id.eq(&p_user_uuid))
			.load::<Data>(&connection.pg_connection)?;

		Ok(docs
			.into_iter()
			.map(|data| Document { connection, data })
			.collect())
	}

	pub fn can_be_viewed_by(self: &Document<'a>, p_user_id: &UserID) -> bool {
		self.data.user_id.eq(&**p_user_id)
	}

	pub fn can_be_edited_by(self: &Document<'a>, p_user_id: &UserID) -> bool {
		self.data.user_id.eq(&**p_user_id)
	}

	pub fn is_owned_by(&self, p_user_id: &UserID) -> bool {
		self.data.user_id.eq(&**p_user_id)
	}

	pub fn get_items(self: &Document<'a>) -> QueryResult<Vec<super::item::Item<'a>>> {
		super::item::Item::get_by_document(self.connection, &self.get_id())
	}

	pub fn get_root(&self) -> QueryResult<Item> {
		let root_id = self
			.data
			.root_item_id
			.ok_or(diesel::result::Error::NotFound)?;

		Item::get_by_id(&self.connection, &ItemID::from_uuid(root_id))
	}

	pub fn get_title(&self) -> QueryResult<String> {
		Ok(self.get_root()?.data.item_text)
	}

	pub fn rename(&mut self, update_text: &str) -> QueryResult<()> {
		let mut root_item = self.get_root()?;
		root_item.update_text(update_text)?;

		Ok(())
	}

	fn get_serialized_title(&self) -> Option<String> {
		if let Ok(title) = self.get_title() {
			if title == "" {
				Some("Untitled Document".to_string())
			} else {
				Some(title)
			}
		} else {
			None
		}
	}

	fn get_serialized_root_id(&self) -> Option<ItemID> {
		if let Some(root_id) = self.data.root_item_id {
			Some(ItemID::from_uuid(root_id))
		} else {
			None
		}
	}

	pub fn delete(&mut self) -> QueryResult<usize> {
		diesel::delete(documents)
			.filter(data::schema::documents::dsl::id.eq(&self.data.id))
			.execute(&self.connection.pg_connection)
	}

	pub fn serialize_with_items(&'a self) -> QueryResult<DocumentWithItems<'a>> {
		let items_hashmap = self.get_items()?.into_iter().fold(
			std::collections::HashMap::new(),
			|mut acc, i| {
				acc.entry(i.get_id()).or_insert(i);
				acc
			},
		);

		Ok(DocumentWithItems::<'a> {
			document: &self,
			items_hashmap,
		})
	}
}

fn serialize_document<'a, S>(
	document: &Document<'a>,
	serializer: S,
	items_hashmap: Option<&HashMap<ItemID, Item>>,
) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	let mut serialized =
		serializer.serialize_struct("Document", if items_hashmap.is_some() { 6 } else { 5 })?;

	serialized.serialize_field("document_id", &document.get_id())?;
	serialized.serialize_field("title", &document.get_serialized_title())?;
	serialized.serialize_field("root_item_id", &document.get_serialized_root_id())?;
	serialized.serialize_field("created_at", &document.data.created_at)?;
	serialized.serialize_field("modified_at", &document.data.modified_at)?;

	if let Some(ser_items) = items_hashmap {
		serialized.serialize_field("items", ser_items)?;
	}

	serialized.end()
}

impl<'a> Serialize for Document<'a> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serialize_document(self, serializer, None)
	}
}

impl<'a> Serialize for DocumentWithItems<'a> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serialize_document(self.document, serializer, Some(&self.items_hashmap))
	}
}
