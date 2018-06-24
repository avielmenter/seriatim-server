use data::db::Connection;
use data::user;
use data::user::User;

use rocket::Route;

use routes::error::Error;
use routes::io::{send_success, SeriatimResult};

use std;

#[get("/current")]
fn current_user(connection: Connection, user_id: user::UserID) -> SeriatimResult {
	let u = User::get_by_id(&connection, &user_id)?;
	Ok(send_success(&u))
}

#[get("/documents")]
fn list_documents(connection: Connection, user_id: user::UserID) -> SeriatimResult {
	let u = User::get_by_id(&connection, &user_id)?;
	let docs = u.get_documents()?;

	Ok(send_success(&docs))
}

#[get("/<path..>")]
fn not_logged_in(path: std::path::PathBuf) -> SeriatimResult {
	Err(Error::NotLoggedIn)
}

pub fn routes() -> Vec<Route> {
	routes![current_user, list_documents, not_logged_in]
}
