use data;
use data::db::Connection;
use data::schema::documents::dsl::{documents, user_id};
use data::schema::users;
use data::schema::users::dsl::*;

use diesel;
use diesel::prelude::*;
use diesel::Connection as DieselConnection;

use oauth::facebook::FacebookUser;
use oauth::google::GoogleUser;
use oauth::twitter::TwitterUser;
use oauth::{LoginMethod, OAuthUser};

use serde::ser::{Serialize, SerializeStruct, Serializer};

use uuid;

#[derive(TaggedID)]
pub struct UserID(uuid::Uuid);

pub struct User<'a> {
    connection: &'a Connection,
    pub data: Data,
}

#[derive(Insertable)]
#[table_name = "users"]
struct NewUser {
    pub display_name: String,
    pub google_id: Option<String>,
    pub twitter_screen_name: Option<String>,
    pub facebook_id: Option<String>,
}

#[derive(Debug, AsChangeset, Queryable, Identifiable)]
#[table_name = "users"]
pub struct Data {
    id: uuid::Uuid,
    pub display_name: String,
    pub google_id: Option<String>,
    pub twitter_screen_name: Option<String>,
    pub facebook_id: Option<String>,
}

impl<'a> User<'a> {
    pub fn get_id(&self) -> UserID {
        UserID(self.data.id.clone())
    }

    pub fn get_by_id(connection: &'a Connection, p_user_id: &UserID) -> QueryResult<User<'a>> {
        let p_uuid = **p_user_id;

        let data = users
            .filter(id.eq(&p_uuid))
            .first::<Data>(&connection.pg_connection)?;

        Ok(User { connection, data })
    }

    pub fn get_by_twitter(
        connection: &'a Connection,
        twitter_user: &TwitterUser,
    ) -> QueryResult<User<'a>> {
        let data = users
            .filter(twitter_screen_name.eq(&twitter_user.screen_name))
            .first::<Data>(&connection.pg_connection)?;

        Ok(User { connection, data })
    }

    pub fn get_by_google(
        connection: &'a Connection,
        google_user: &GoogleUser,
    ) -> QueryResult<User<'a>> {
        let data = users
            .filter(google_id.eq(&google_user.id))
            .first::<Data>(&connection.pg_connection)?;

        Ok(User { connection, data })
    }

    pub fn get_by_facebook(
        connection: &'a Connection,
        facebook_user: &FacebookUser,
    ) -> QueryResult<User<'a>> {
        let data = users
            .filter(facebook_id.eq(&facebook_user.id))
            .first::<Data>(&connection.pg_connection)?;

        Ok(User { connection, data })
    }

    pub fn get_by_oauth_user(
        connection: &'a Connection,
        oauth_user: &OAuthUser,
    ) -> QueryResult<User<'a>> {
        match oauth_user {
            OAuthUser::Google(google_user) => User::get_by_google(connection, google_user),
            OAuthUser::Twitter(twitter_user) => User::get_by_twitter(connection, twitter_user),
            OAuthUser::Facebook(facebook_user) => User::get_by_facebook(connection, facebook_user),
        }
    }

    pub fn create_from_twitter(
        connection: &'a Connection,
        twitter_user: &TwitterUser,
    ) -> QueryResult<User<'a>> {
        let data = diesel::insert_into(users)
            .values(NewUser {
                display_name: twitter_user.name.clone(),
                google_id: None,
                facebook_id: None,
                twitter_screen_name: Some(twitter_user.screen_name.clone()),
            })
            .get_result(&connection.pg_connection)?;

        Ok(User { connection, data })
    }

    pub fn create_from_google(
        connection: &'a Connection,
        google_user: &GoogleUser,
    ) -> QueryResult<User<'a>> {
        let data = diesel::insert_into(users)
            .values(NewUser {
                display_name: google_user.name.clone(),
                google_id: Some(google_user.id.clone()),
                twitter_screen_name: None,
                facebook_id: None,
            })
            .get_result(&connection.pg_connection)?;

        Ok(User { connection, data })
    }

    pub fn create_from_facebook(
        connection: &'a Connection,
        facebook_user: &FacebookUser,
    ) -> QueryResult<User<'a>> {
        let data = diesel::insert_into(users)
            .values(NewUser {
                display_name: facebook_user.name.clone(),
                facebook_id: Some(facebook_user.id.clone()),
                twitter_screen_name: None,
                google_id: None,
            })
            .get_result(&connection.pg_connection)?;

        Ok(User { connection, data })
    }

    pub fn create_from_oauth_user(
        connection: &'a Connection,
        oauth_user: &OAuthUser,
    ) -> QueryResult<User<'a>> {
        match oauth_user {
            OAuthUser::Google(google_user) => User::create_from_google(connection, google_user),
            OAuthUser::Twitter(twitter_user) => User::create_from_twitter(connection, twitter_user),
            OAuthUser::Facebook(facebook_user) => {
                User::create_from_facebook(connection, facebook_user)
            }
        }
    }

    pub fn get_documents(self: &User<'a>) -> QueryResult<Vec<data::document::Document<'a>>> {
        data::document::Document::get_by_user(self.connection, &self.get_id())
    }

    pub fn create_document(&self) -> QueryResult<data::document::Document> {
        data::document::Document::create_for_user(&self.connection, &self.get_id())
    }

    pub fn has_facebook(&self) -> bool {
        self.data.facebook_id.clone().unwrap_or("".to_string()) != "".to_string()
    }

    pub fn has_twitter(&self) -> bool {
        self.data
            .twitter_screen_name
            .clone()
            .unwrap_or("".to_string())
            != "".to_string()
    }

    pub fn has_google(&self) -> bool {
        self.data.google_id.clone().unwrap_or("".to_string()) != "".to_string()
    }

    fn get_merged_user_data(lhs: &Data, rhs: &Data) -> Data {
        Data {
            id: lhs.id.clone(),
            display_name: lhs.display_name.clone(),
            google_id: if lhs.google_id.is_some() {
                lhs.google_id.clone()
            } else {
                rhs.google_id.clone()
            },
            facebook_id: if lhs.facebook_id.is_some() {
                lhs.facebook_id.clone()
            } else {
                rhs.facebook_id.clone()
            },
            twitter_screen_name: if lhs.twitter_screen_name.is_some() {
                lhs.twitter_screen_name.clone()
            } else {
                rhs.twitter_screen_name.clone()
            },
        }
    }

    pub fn merge<'b>(&mut self, merge_user: &User<'b>) -> QueryResult<&mut User<'a>> {
        if (self.has_facebook() && merge_user.has_facebook())
            || (self.has_twitter() && merge_user.has_twitter())
            || (self.has_google() && merge_user.has_google())
        {
            return Err(diesel::result::Error::QueryBuilderError(Box::new(
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Cannot merge two users with overlapping authentication methods.",
                ),
            )));
        }

        let merged_user_data = User::get_merged_user_data(&self.data, &merge_user.data);

        let new_data = self
            .connection
            .pg_connection
            .transaction::<_, diesel::result::Error, _>(|| {
                diesel::update(documents)
                    .filter(user_id.eq(&merge_user.data.id))
                    .set(user_id.eq(&self.data.id))
                    .execute(&self.connection.pg_connection)?;

                diesel::delete(users)
                    .filter(id.eq(&merge_user.data.id))
                    .execute(&self.connection.pg_connection)?;

                Ok(diesel::update(users)
                    .filter(id.eq(self.data.id))
                    .set(&merged_user_data)
                    .get_result::<Data>(&self.connection.pg_connection)?)
            })?;

        self.data = new_data;

        Ok(self)
    }

    pub fn update_display_name(&mut self, new_display_name: &str) -> QueryResult<&mut User<'a>> {
        let new_data = diesel::update(users)
            .filter(id.eq(&self.data.id))
            .set(display_name.eq(new_display_name))
            .get_result::<Data>(&self.connection.pg_connection)?;

        self.data = new_data;

        Ok(self)
    }

    pub fn count_login_methods(&self) -> u8 {
        (if self.has_facebook() { 1 } else { 0 })
            + (if self.has_google() { 1 } else { 0 })
            + (if self.has_twitter() { 1 } else { 0 })
    }

    pub fn remove_login_method(&mut self, method: &LoginMethod) -> QueryResult<&mut User<'a>> {
        if self.count_login_methods() <= 1 {
            return Ok(self);
        }

        let update_filter = diesel::update(users).filter(id.eq(self.data.id.clone()));

        let new_data = match method {
            LoginMethod::Twitter => update_filter
                .set(twitter_screen_name.eq(None::<String>))
                .get_result::<Data>(&self.connection.pg_connection),
            LoginMethod::Facebook => update_filter
                .set(facebook_id.eq(None::<String>))
                .get_result::<Data>(&self.connection.pg_connection),
            LoginMethod::Google => update_filter
                .set(google_id.eq(None::<String>))
                .get_result::<Data>(&self.connection.pg_connection),
        }?;

        self.data = new_data;

        Ok(self)
    }
}

impl<'a> Serialize for User<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serialized = serializer.serialize_struct("User", 3)?;
        serialized.serialize_field("user_id", &self.get_id())?;
        serialized.serialize_field("display_name", &self.data.display_name)?;
        serialized.serialize_field("facebook_id", &self.data.facebook_id)?;
        serialized.serialize_field("google_id", &self.data.google_id)?;
        serialized.serialize_field("twitter_screen_name", &self.data.twitter_screen_name)?;

        serialized.end()
    }
}
