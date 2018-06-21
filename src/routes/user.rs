use data::db::Connection;
use data::user;
use data::user::User;

use rocket::Route;

use routes::io::{send_success, SeriatimResult};

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

pub fn routes() -> Vec<Route> {
	routes![current_user, list_documents]
}
