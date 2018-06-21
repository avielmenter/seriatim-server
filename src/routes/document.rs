use data::db::Connection;
use data::document::{Document, DocumentID};
use data::user;

use routes::error::Error;
use routes::io::{send_success, SeriatimResult};

use rocket::Route;
use rocket_contrib::Json;

#[get("/create")]
fn create_document(connection: Connection, user_id: user::UserID) -> SeriatimResult {
	let u = user::User::get_by_id(&connection, &user_id)?;
	let doc = u.create_document()?;

	Ok(send_success(&doc.serialize_with_items()?))
}

#[delete("/<doc_id>/delete")]
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

#[derive(Deserialize)]
struct RenameDocumentParams {
	name: String,
}

#[post("/<doc_id>/rename", format = "application/json", data = "<rename>")]
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

	doc.rename(&rename.name)?;
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

pub fn routes() -> Vec<Route> {
	routes![
		create_document,
		delete_document,
		rename_document,
		get_document
	]
}
