use data::category::Category;
use data::db::Connection;
use data::document::{Document, DocumentID};
use data::item::ItemID;
use data::schema::{StyleProperty, StyleUnit};
use data::user;

use diesel::result::QueryResult;

use routes::error::Error;
use routes::io::{cors_response, send_success, send_with_permissions, SeriatimResult};

use rocket;
use rocket::Route;
use rocket_contrib::json::Json;

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

	Ok(send_success(&doc.serializable(Some(&user_id))?))
}

#[options("/<_doc_id>")]
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

	if !doc.is_trashed(&user_id)? && doc.can_be_viewed_by(&user_id) {
		println!("MOVING TO TRASH");

		Category::create(&connection, &doc_id, &user_id, Category::TRASH)?;
		Ok(send_success(&doc.serializable(Some(&user_id))?))
	} else if doc.is_owned_by(&user_id) {
		println!("DELETING DOCUMENT");

		doc.delete()?;
		Ok(send_success(&doc.serializable(Some(&user_id))?))
	} else {
		Err(Error::InsufficientPermissions)
	}
}

#[derive(Serialize, Deserialize)]
struct RenameDocumentParams {
	name: String,
}

#[options("/<_doc_id>/rename")]
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
	Ok(send_success(&doc.serializable(Some(&user_id))?))
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
			&doc.serialize_with_items(Some(&user_id))?,
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
			&doc.serialize_with_items(None)?,
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
		Ok(send_success(&new_doc.serializable(Some(&user_id))?))
	} else {
		Err(Error::InsufficientPermissions)
	}
}

#[derive(Serialize, Deserialize)]
struct EditDocumentStyle {
	property: StyleProperty,
	value_string: Option<String>,
	value_number: Option<i32>,
	unit: Option<StyleUnit>,
}

#[derive(Serialize, Deserialize)]
struct EditDocumentItem {
	item_id: String,
	parent_id: Option<String>,
	child_order: i32,
	children: Vec<String>,
	item_text: Option<String>,
	styles: Vec<EditDocumentStyle>,
}

#[derive(Serialize, Deserialize)]
struct EditDocumentParams {
	root_item: String,
	toc_item: Option<String>,
	items: HashMap<String, EditDocumentItem>,
}

#[options("/<_doc_id>/edit")]
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
			let mut new_item = doc.add_item(
				Some(parent_uuid),
				curr_item.child_order,
				curr_item.item_text.clone(),
			)?;

			let db_styles = curr_item
				.styles
				.iter()
				.map(|s| {
					new_item.create_style(
						s.property,
						s.value_string.clone(),
						s.value_number,
						s.unit,
					)
				})
				.collect();

			new_item.update_styles(db_styles)?;

			curr_item_id = new_item.get_id();
		}
	}

	let mut child_ids = curr_item // handle children
		.children
		.iter()
		.map(|child_id| merge_edit_subtree(doc, subtree, child_id, Some(&curr_item_id.json_str())))
		.fold(
			Ok(HashMap::<String, Option<ItemID>>::new()),
			|prev_result: QueryResult<HashMap<String, Option<ItemID>>>, curr_children| {
				let mut prev = prev_result?;

				for (curr_child_id, curr_child) in curr_children?.into_iter() {
					prev.insert(curr_child_id, curr_child);
				}

				Ok(prev)
			},
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

	println!("UPDATED ITEMS");

	let toc_item_id = match subtree.toc_item {
		Some(ref t) => id_map.get(t).unwrap_or(&None),
		None => &None,
	};

	println!(
		"SETTING TOC ITEM ID: {}",
		toc_item_id
			.as_ref()
			.and_then(|t| Some(t.hyphenated().to_string()))
			.unwrap_or("".to_string())
	);

	doc.set_toc_item(toc_item_id)?.touch()?;
	Ok(send_success(&id_map))
}

#[options("/<_doc_id>/edit_text")]
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

#[options("/<_doc_id>/public_viewability")]
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
			&doc.set_publicly_viewable(viewability.publicly_viewable)?
				.serializable(Some(&user_id))?,
		))
	}
}

#[derive(Serialize, Deserialize)]
struct AddCategoryParams {
	name: String,
}

#[options("/<_doc_id>/categories")]
fn category_options<'a>(_doc_id: DocumentID) -> rocket::response::Response<'a> {
	cors_response::<'a>()
}

#[post("/<doc_id>/categories", format = "json", data = "<new_category>")]
fn add_category(
	doc_id: DocumentID,
	connection: Connection,
	user_id: user::UserID,
	new_category: Json<AddCategoryParams>,
) -> SeriatimResult {
	let doc = Document::get_by_id(&connection, &doc_id)?;

	if !doc.can_be_viewed_by(&user_id) {
		Err(Error::InsufficientPermissions)
	} else {
		Category::create(&connection, &doc_id, &user_id, &new_category.name)?;
		Ok(send_success(&doc.serializable(Some(&user_id))?))
	}
}

#[options("/<_doc_id>/categories/<_cat_name>")]
fn delete_category_options<'a>(
	_doc_id: DocumentID,
	_cat_name: String,
) -> rocket::response::Response<'a> {
	cors_response::<'a>()
}

#[delete("/<doc_id>/categories/<cat_name>")]
fn delete_category(
	doc_id: DocumentID,
	connection: Connection,
	user_id: user::UserID,
	cat_name: String,
) -> SeriatimResult {
	let doc = Document::get_by_id(&connection, &doc_id)?;

	if !doc.can_be_viewed_by(&user_id) {
		Err(Error::InsufficientPermissions)
	} else {
		let mut category = Category::get_category(&connection, &doc_id, &user_id, &cat_name)?;
		category.delete()?;

		Ok(send_success(&doc.serializable(Some(&user_id))?))
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
		category_options,
		add_category,
		delete_category_options,
		delete_category,
		not_logged_in_get,
		not_logged_in_post,
	]
}
