use data::db::Connection;
use data::document::{Document, DocumentID};
use data::item::ItemID;
use data::user;

use diesel::result::QueryResult;

use routes::error::Error;
use routes::io::{cors_response, send_success, send_with_permissions, SeriatimResult};

use rocket;
use rocket::Route;
use rocket_contrib::Json;

use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Serialize)]
struct DocumentPermissions {
	edit: bool,
}

#[post("/create")]
fn create_document(connection: Connection, user_id: user::UserID) -> SeriatimResult {
	let u = user::User::get_by_id(&connection, &user_id)?;
	let doc = u.create_document()?;

	Ok(send_success(&doc))
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
		Ok(send_with_permissions(
			&doc.serialize_with_items()?,
			&DocumentPermissions {
				edit: doc.can_be_edited_by(&user_id),
			},
		))
	} else {
		Err(Error::InsufficientPermissions)
	}
}

#[get("/<doc_id>", rank = 1)]
fn get_anonymously(doc_id: DocumentID, connection: Connection) -> SeriatimResult {
	let doc = Document::get_by_id(&connection, &doc_id)?;

	if doc.can_be_viewed_anonymously() {
		Ok(send_with_permissions(
			&doc.serialize_with_items()?,
			&DocumentPermissions { edit: false },
		))
	} else {
		Err(Error::InsufficientPermissions)
	}
}

#[post("/<doc_id>/copy")]
fn copy_document(
	doc_id: DocumentID,
	connection: Connection,
	user_id: user::UserID,
) -> SeriatimResult {
	let doc = Document::get_by_id(&connection, &doc_id)?;

	if doc.can_be_viewed_by(&user_id) {
		let new_doc = doc.copy_to_user(&user_id)?;
		Ok(send_success(&new_doc))
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
	item_text: Option<String>,
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

fn merge_edit_subtree<'a>(
	doc: &mut Document<'a>,
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

	if let Some(ref parent_id_str) = parent {
		// add this item, unless it's the root of the subtree
		if let Ok(parent_uuid) = ItemID::from_str(&parent_id_str) {
			let new_item = doc.add_item(
				Some(parent_uuid),
				curr_item.child_order,
				curr_item.item_text.clone(),
			)?;

			curr_item_id = new_item.get_id();
		}
	}

	let mut child_ids = curr_item	// handle children
		.children
		.iter()
		.map(|child_id| merge_edit_subtree(doc, subtree, child_id, Some(&curr_item_id.json_str())))
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

fn update_root(doc: &mut Document, subtree: &EditDocumentParams) -> Result<(), Error> {
	let mut root_item = doc.get_item(&ItemID::from_str(&subtree.root_item)?)?;
	root_item.remove_children()?;

	if let Some(root_edit_item) = subtree.items.get(&subtree.root_item) {
		if let Some(ref text) = root_edit_item.item_text {
			root_item.update_text(&text)?;
		}
	}

	Ok(())
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

	update_root(&mut doc, &subtree)?;

	let id_map = merge_edit_subtree(&mut doc, &subtree, &subtree.root_item, None)?;

	doc.touch()?;
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
	let mut doc = Document::get_by_id(&connection, &doc_id)?;

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

		doc.touch()?;
		Ok(send_success(&()))
	}
}

#[derive(Serialize, Deserialize)]
struct DocumentViewabilityParams {
	publicly_viewable: bool,
}

#[route(OPTIONS, "/<_doc_id>/public_viewability")]
fn public_viewability_options<'a>(_doc_id: DocumentID) -> rocket::response::Response<'a> {
	cors_response::<'a>()
}

#[post(
	"/<doc_id>/public_viewability",
	format = "json",
	data = "<viewability>"
)]
fn public_viewability(
	doc_id: DocumentID,
	connection: Connection,
	user_id: user::UserID,
	viewability: Json<DocumentViewabilityParams>,
) -> SeriatimResult {
	let mut doc = Document::get_by_id(&connection, &doc_id)?;

	if !doc.is_owned_by(&user_id) {
		Err(Error::InsufficientPermissions)
	} else {
		Ok(send_success(
			&doc.set_publicly_viewable(viewability.publicly_viewable)?,
		))
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
		get_anonymously,
		copy_document,
		edit_options,
		edit_document,
		edit_text_options,
		edit_document_text,
		public_viewability_options,
		public_viewability,
		not_logged_in_get,
		not_logged_in_post,
	]
}
