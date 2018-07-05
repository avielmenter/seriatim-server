use data::db::Connection;
use data::document::{Document, DocumentID};
use data::item::{Item, ItemID};
use data::user;

use diesel::result::QueryResult;

use routes::error::Error;
use routes::io::{cors_response, send_success, SeriatimResult};

use rocket;
use rocket::Route;
use rocket_contrib::Json;

use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

#[post("/create")]
fn create_document(connection: Connection, user_id: user::UserID) -> SeriatimResult {
	let u = user::User::get_by_id(&connection, &user_id)?;
	let doc = u.create_document()?;

	Ok(send_success(&doc.serialize_with_items()?))
}

#[route(OPTIONS, "/<_doc_id>")]
fn delete_options<'a>(_doc_id: DocumentID) -> rocket::response::Response<'a> {
	cors_response::<'a>()
}

#[delete("/<doc_id>")]
fn delete_document(
	doc_id: DocumentID,
	connection: Connection,
	user_id: user::UserID,
) -> SeriatimResult {
	let mut doc = Document::get_by_id(&connection, &doc_id)?;
	if !doc.is_owned_by(&user_id) {
		return Err(Error::InsufficientPermissions);
	}

	doc.delete()?;
	Ok(send_success(&doc))
}

#[derive(Serialize, Deserialize)]
struct RenameDocumentParams {
	name: String,
}

#[route(OPTIONS, "/<_doc_id>/rename")]
fn rename_options<'a>(_doc_id: DocumentID) -> rocket::response::Response<'a> {
	cors_response::<'a>()
}

#[post("/<doc_id>/rename", format = "json", data = "<rename>")]
fn rename_document(
	doc_id: DocumentID,
	rename: Json<RenameDocumentParams>,
	connection: Connection,
	user_id: user::UserID,
) -> SeriatimResult {
	let mut doc = Document::get_by_id(&connection, &doc_id)?;
	if !doc.can_be_edited_by(&user_id) {
		return Err(Error::InsufficientPermissions);
	}

	doc.rename(&rename.0.name)?;
	Ok(send_success(&doc))
}

#[get("/<doc_id>")]
fn get_document(
	doc_id: DocumentID,
	connection: Connection,
	user_id: user::UserID,
) -> SeriatimResult {
	let doc = Document::get_by_id(&connection, &doc_id)?;

	if doc.can_be_viewed_by(&user_id) {
		Ok(send_success(&doc.serialize_with_items()?))
	} else {
		Err(Error::InsufficientPermissions)
	}
}

#[derive(Serialize, Deserialize)]
struct EditDocumentItem {
	item_id: String,
	parent_id: Option<String>,
	child_order: i32,
	children: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct EditDocumentParams {
	root_item: String,
	items: HashMap<String, EditDocumentItem>,
}

#[route(OPTIONS, "/<_doc_id>/edit")]
fn edit_options<'a>(_doc_id: DocumentID) -> rocket::response::Response<'a> {
	cors_response::<'a>()
}

fn get_existing_items<'a>(doc: &Document<'a>) -> QueryResult<HashMap<ItemID, Item<'a>>> {
	let existing_items = doc
		.get_items()?
		.into_iter()
		.map(|i| (i.get_id(), i))
		.collect::<HashMap<ItemID, Item>>();

	Ok(existing_items)
}

fn remove_children<'a>(
	doc: &Document<'a>,
	existing_items: &mut HashMap<ItemID, Item<'a>>,
	children: &Vec<String>,
	curr: &str,
) -> QueryResult<()> {
	let mut items_to_remove = Vec::<ItemID>::new();

	for (existing_id, existing_item) in existing_items.iter_mut() {
		// remove nonexistent children
		let parent_id = existing_item.get_parent_id();

		if parent_id.is_some()
			&& parent_id.unwrap().json_str() == curr
			&& !children.contains(&existing_id.json_str())
		{
			existing_item.remove()?;
			items_to_remove.push(existing_item.get_id());
		}
	}

	if items_to_remove.len() > 0 {
		let updated_items = get_existing_items(&doc)?; // we need to refresh existing items from the database because removing an item could cause cascading deletes

		existing_items.clear();
		existing_items.extend(updated_items);
	}

	Ok(())
}

fn merge_edit_subtree<'a>(
	doc: &mut Document<'a>,
	existing_items: &mut HashMap<ItemID, Item<'a>>,
	subtree: &EditDocumentParams,
	curr: &str,
	parent: Option<&str>,
) -> QueryResult<HashMap<String, Option<ItemID>>> {
	let curr_item = subtree.items.get(curr);
	let curr_item_id = ItemID::from_str(curr);

	if curr_item.is_none() || curr_item_id.is_err() {
		let mut ret = HashMap::<String, Option<ItemID>>::new();
		ret.insert(curr.to_string(), None);

		return Ok(ret);
	}

	let curr_item = curr_item.unwrap();
	let mut curr_item_id = curr_item_id.unwrap();

	if !existing_items.contains_key(&curr_item_id) {
		// add this item to db if necessary
		if let Some(ref parent_id_str) = parent {
			if let Ok(parent_uuid) = ItemID::from_str(&parent_id_str) {
				let new_item = doc.add_item(Some(parent_uuid), curr_item.child_order)?;
				curr_item_id = new_item.get_id();
			}
		}
	}

	remove_children(&doc, existing_items, &curr_item.children, curr)?;

	let mut child_ids = curr_item	// handle children
		.children
		.iter()
		.map(|child_id| merge_edit_subtree(doc, existing_items, subtree, child_id, Some(&curr_item_id.json_str())))
		.fold(Ok(HashMap::<String, Option<ItemID>>::new()),
			|prev_result : QueryResult<HashMap<String, Option<ItemID>>>, curr_children| {
				let mut prev = prev_result?;

				for (curr_child_id, curr_child) in curr_children?.into_iter() {
					prev.insert(curr_child_id, curr_child);
				}

				Ok(prev)
			}
		)?;

	child_ids.insert(curr.to_string(), Some(curr_item_id));

	Ok(child_ids)
}

#[post("/<doc_id>/edit", format = "json", data = "<subtree>")]
fn edit_document(
	doc_id: DocumentID,
	connection: Connection,
	user_id: user::UserID,
	subtree: Json<EditDocumentParams>,
) -> SeriatimResult {
	let mut doc = Document::get_by_id(&connection, &doc_id)?;

	if !doc.can_be_edited_by(&user_id) {
		return Err(Error::InsufficientPermissions);
	}

	let mut existing_items = get_existing_items(&doc)?;

	let id_map = merge_edit_subtree(
		&mut doc,
		&mut existing_items,
		&subtree,
		&subtree.root_item,
		None,
	)?;

	Ok(send_success(&id_map))
}

#[route(OPTIONS, "/<_doc_id>/edit_text")]
fn edit_text_options<'a>(_doc_id: DocumentID) -> rocket::response::Response<'a> {
	cors_response::<'a>()
}

#[post("/<doc_id>/edit_text", format = "json", data = "<changes>")]
fn edit_document_text(
	doc_id: DocumentID,
	connection: Connection,
	user_id: user::UserID,
	changes: Json<HashMap<String, String>>,
) -> SeriatimResult {
	let doc = Document::get_by_id(&connection, &doc_id)?;

	if !doc.can_be_edited_by(&user_id) {
		Err(Error::InsufficientPermissions)
	} else {
		let mut items = doc.get_items()?;
		for item in items.iter_mut() {
			let item_id = item.get_id().json_str();

			if let Some(new_text) = changes.get(&item_id) {
				item.update_text(&new_text)?;
			}
		}

		Ok(send_success(&()))
	}
}

#[get("/<_path..>", rank = 2)]
fn not_logged_in_get(_path: PathBuf) -> SeriatimResult {
	Err(Error::NotLoggedIn)
}

#[post("/<_path..>", rank = 2)]
fn not_logged_in_post(_path: PathBuf) -> SeriatimResult {
	Err(Error::NotLoggedIn)
}

pub fn routes() -> Vec<Route> {
	routes![
		create_document,
		delete_options,
		delete_document,
		rename_document,
		rename_options,
		get_document,
		edit_options,
		edit_document,
		edit_text_options,
		edit_document_text,
		not_logged_in_get,
		not_logged_in_post,
	]
}
