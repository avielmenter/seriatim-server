use data::db::Connection;
use data::document::SerializableDocument;
use data::user;
use data::user::User;

use diesel::result::QueryResult;

use oauth::LoginMethod;

use rocket::{self, Route};
use rocket_contrib::json::Json;

use routes::error::Error;
use routes::io::{cors_response, send_success, SeriatimResult};

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

	let serializable_docs = docs
		.iter()
		.map(|d| d.serializable(Some(&user_id)))
		.collect::<QueryResult<Vec<SerializableDocument>>>()?;

	Ok(send_success(&serializable_docs))
}

#[derive(Deserialize)]
struct UpdateUserParams {
	display_name: String,
}

#[options("/update")]
fn update_options<'a>() -> rocket::response::Response<'a> {
	cors_response::<'a>()
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

#[post("/remove_login/<login_method>")]
fn remove_login(
	con: Connection,
	user_id: user::UserID,
	login_method: LoginMethod,
) -> SeriatimResult {
	let mut u = User::get_by_id(&con, &user_id)?;

	if u.count_login_methods() <= 1 {
		Err(Error::TooFewLoginMethods)
	} else {
		Ok(send_success(u.remove_login_method(&login_method)?))
	}
}

#[get("/<_path..>")]
fn not_logged_in(_path: std::path::PathBuf) -> SeriatimResult {
	Err(Error::NotLoggedIn)
}

pub fn routes() -> Vec<Route> {
	routes![
		current_user,
		list_documents,
		update_options,
		update_user,
		remove_login,
		not_logged_in
	]
}
