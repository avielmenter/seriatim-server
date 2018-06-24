use data::db::Connection;
use data::document::{Document, DocumentID};
use data::user;

use routes::error::Error;
use routes::io::{send_success, SeriatimResult};

use rocket;
use rocket::Route;
use rocket_contrib::Json;

use std::path::PathBuf;

#[post("/create")]
fn create_document(connection: Connection, user_id: user::UserID) -> SeriatimResult {
	let u = user::User::get_by_id(&connection, &user_id)?;
	let doc = u.create_document()?;

	Ok(send_success(&doc.serialize_with_items()?))
}

#[post("/<doc_id>/delete")]
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

#[route(OPTIONS, "/<doc_id>/rename")]
fn rename_options<'a>(doc_id: DocumentID) -> rocket::response::Response<'a> {
	rocket::response::Response::build()
		.raw_header(
			"Access-Control-Allow-Origin",
			dotenv!("SERIATIM_ALLOWED_ORIGIN"),
		)
		.raw_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
		.raw_header("Access-Control-Allow-Headers", "Content-Type")
		.finalize()
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
		delete_document,
		rename_document,
		rename_options,
		get_document,
		not_logged_in_get,
		not_logged_in_post,
	]
}
