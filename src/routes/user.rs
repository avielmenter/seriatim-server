use data::db::Connection;
use data::user;
use data::user::User;

use rocket::Route;
use rocket_contrib::Json;

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

#[derive(Deserialize)]
struct UpdateUserParams {
	display_name: String,
}

#[post("/update", format = "json", data = "<update_params>")]
fn update_user(
	con: Connection,
	user_id: user::UserID,
	update_params: Json<UpdateUserParams>,
) -> SeriatimResult {
	let mut u = User::get_by_id(&con, &user_id)?;
	u.update_display_name(&update_params.display_name)?;

	Ok(send_success(&u))
}

#[get("/<_path..>")]
fn not_logged_in(_path: std::path::PathBuf) -> SeriatimResult {
	Err(Error::NotLoggedIn)
}

pub fn routes() -> Vec<Route> {
	routes![current_user, list_documents, update_user, not_logged_in]
}
