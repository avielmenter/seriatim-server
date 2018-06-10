use diesel;
use diesel::prelude::*;

use data::db::Connection;
use data::schema::items;
use data::schema::items::dsl::*;

use uuid;

pub struct Item<'a> {
	connection: &'a Connection,
	data: Data,
}

#[derive(Identifiable, AsChangeset, Queryable)]
#[table_name = "items"]
pub struct Data {
	pub id: uuid::Uuid,
	pub document_id: uuid::Uuid,
	pub parent_id: Option<uuid::Uuid>,
	pub item_text: String,
	pub collapsed: bool,
}

#[derive(Insertable)]
#[table_name = "items"]
pub struct NewItem<'a> {
	pub document_id: uuid::Uuid,
	pub parent_id: Option<uuid::Uuid>,
	pub item_text: &'a str,
	pub collapsed: bool,
}

impl<'a> Item<'a> {
	fn results_list(
		connection: &'a Connection,
		items_list: Vec<Data>,
	) -> QueryResult<Vec<Item<'a>>> {
		Ok(items_list
			.into_iter()
			.map(|data| Item { connection, data })
			.collect())
	}

	pub fn get_by_id(connection: &'a Connection, p_item_id: &uuid::Uuid) -> QueryResult<Item<'a>> {
		let data = items
			.filter(id.eq(&p_item_id))
			.first::<Data>(&connection.pg_connection)?;

		Ok(Item { connection, data })
	}

	pub fn get_by_document(
		connection: &'a Connection,
		p_document_id: &uuid::Uuid,
	) -> QueryResult<Vec<Item<'a>>> {
		let items_list = items
			.filter(document_id.eq(&p_document_id))
			.load::<Data>(&connection.pg_connection)?;

		Self::results_list(connection, items_list)
	}

	pub fn create_item(connection: &'a Connection, p_new_item: NewItem) -> QueryResult<Item<'a>> {
		let data = diesel::insert_into(items)
			.values(p_new_item)
			.get_result(&connection.pg_connection)?;

		Ok(Item { connection, data })
	}

	pub fn get_from_parent(
		connection: &'a Connection,
		p_parent_id: &uuid::Uuid,
	) -> QueryResult<Vec<Item<'a>>> {
		let items_list = items
			.filter(parent_id.eq(&p_parent_id))
			.load::<Data>(&connection.pg_connection)?;

		Self::results_list(connection, items_list)
	}

	pub fn get_children(self: &Item<'a>) -> QueryResult<Vec<Item<'a>>> {
		Self::get_from_parent(self.connection, &self.data.id)
	}

	pub fn remove_item(connection: &'a Connection, p_item_id: &uuid::Uuid) -> QueryResult<usize> {
		let deleted_items = diesel::delete(items)
			.filter(id.eq(&p_item_id))
			.execute(&connection.pg_connection)?;

		let child_ids = Self::get_from_parent(connection, p_item_id)?
			.iter()
			.map(|child| child.data.id)
			.collect::<Vec<uuid::Uuid>>();

		child_ids
			.iter()
			.fold(Ok(deleted_items), |sum, child_id| match sum {
				Ok(s) => Ok(s + Self::remove_item(connection, child_id)?), // if we can remove all deleted children, count all we've removed
				Err(e) => Err(e),                                          // otherwise, return an error
			})
	}

	pub fn update_item(
		connection: &'a Connection,
		p_item: &Item,
		children: &[uuid::Uuid],
	) -> QueryResult<Item<'a>> {
		let data = diesel::update(items)
			.set(&p_item.data)
			.get_result(&connection.pg_connection)?;

		let db_children = p_item.get_children()?;

		for db_child in db_children {
			// remove items in database that are no longer children of this item
			if !children.iter().any(|c| c.eq(&db_child.data.id)) {
				Self::remove_item(connection, &db_child.data.id)?;
			}
		}

		Ok(Item { connection, data })
	}
}
