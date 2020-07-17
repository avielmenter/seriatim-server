use diesel;
use diesel::prelude::*;

use data::db::Connection;
use data::document::DocumentID;
use data::schema::categories;
use data::schema::categories::dsl::*;
use data::user::UserID;

use uuid;

use serde::ser::{Serialize, SerializeStruct, Serializer};

#[derive(TaggedID, Serialize, Deserialize)]
pub struct CategoryID(uuid::Uuid);

pub struct Category<'a> {
    connection: &'a Connection,
    pub data: Data,
}

#[derive(Debug, Queryable, Identifiable)]
#[table_name = "categories"]
pub struct Data {
    id: uuid::Uuid,
    document_id: uuid::Uuid,
    user_id: uuid::Uuid,
    pub category_name: String,
}

#[derive(Insertable)]
#[table_name = "categories"]
pub struct NewCategory {
    document_id: uuid::Uuid,
    user_id: uuid::Uuid,
    pub category_name: String,
}

impl<'a> Category<'a> {
    pub const TRASH: &'static str = "Trash";
    pub const MAX_NAME_LENGTH: usize = 32;

    // copied from https://stackoverflow.com/a/38406885
    // surprisingly tough in rust, but I guess that's unicode for you
    fn sanitize_name(s: &str) -> String {
        let lowercased = s.to_ascii_lowercase();
        let mut c = lowercased.chars();

        let cased = match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        };

        cased.chars().take(Category::MAX_NAME_LENGTH).collect()
    }

    fn results_list(
        connection: &'a Connection,
        categories_list: Vec<Data>,
    ) -> QueryResult<Vec<Self>> {
        Ok(categories_list
            .into_iter()
            .map(|data| Category { connection, data })
            .collect())
    }

    pub fn create(
        connection: &'a Connection,
        p_document_id: &DocumentID,
        p_user_id: &UserID,
        p_name: &str,
    ) -> QueryResult<Self> {
        let p_doc_uuid = **p_document_id;
        let p_user_uuid = **p_user_id;

        let new_category = NewCategory {
            document_id: p_doc_uuid,
            user_id: p_user_uuid,
            category_name: Category::sanitize_name(p_name),
        };

        let data: Data = diesel::insert_into(categories)
            .values(new_category)
            .get_result(&connection.pg_connection)?;

        Ok(Category { data, connection })
    }

    pub fn get_id(&self) -> CategoryID {
        CategoryID::from_uuid(self.data.id.clone())
    }

    pub fn get_document_id(&self) -> DocumentID {
        DocumentID::from_uuid(self.data.document_id.clone())
    }

    pub fn get_user_id(&self) -> UserID {
        UserID::from_uuid(self.data.user_id.clone())
    }

    pub fn get_category(
        connection: &'a Connection,
        p_document_id: &DocumentID,
        p_user_id: &UserID,
        p_name: &str,
    ) -> QueryResult<Self> {
        let p_doc_uuid = **p_document_id;
        let p_user_uuid = **p_user_id;

        let data = categories
            .filter(document_id.eq(&p_doc_uuid))
            .filter(user_id.eq(&p_user_uuid))
            .filter(category_name.eq(&Category::sanitize_name(p_name)))
            .first::<Data>(&connection.pg_connection)?;

        Ok(Category { connection, data })
    }

    pub fn get_categories(
        connection: &'a Connection,
        p_document_id: &DocumentID,
        p_user_id: &UserID,
    ) -> QueryResult<Vec<Self>> {
        let p_doc_uuid = **p_document_id;
        let p_user_uuid = **p_user_id;

        let categories_list = categories
            .filter(document_id.eq(&p_doc_uuid))
            .filter(user_id.eq(&p_user_uuid))
            .load::<Data>(&connection.pg_connection)?;

        Self::results_list(connection, categories_list)
    }

    pub fn delete(&mut self) -> QueryResult<usize> {
        diesel::delete(categories)
            .filter(id.eq(self.data.id))
            .execute(&self.connection.pg_connection)
    }
}

impl<'a> Serialize for Category<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serialized = serializer.serialize_struct("Category", 4)?;

        serialized.serialize_field("id", &self.get_id())?;
        serialized.serialize_field("document_id", &self.get_document_id())?;
        serialized.serialize_field("user_id", &self.get_user_id())?;
        serialized.serialize_field("category_name", &self.data.category_name)?;

        serialized.end()
    }
}
