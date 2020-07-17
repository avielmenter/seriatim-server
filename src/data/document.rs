use diesel;
use diesel::prelude::*;

use data;
use data::category::Category;
use data::db::Connection;
use data::item::{Item, ItemID, ItemWithStyles};
use data::schema::documents;
use data::schema::documents::dsl::*;
use data::schema::items;
use data::schema::items::dsl::*;
use data::user::UserID;

use uuid;

use serde::ser::{Serialize, SerializeStruct, Serializer};

use std::collections::HashMap;
use std::time::SystemTime;

#[derive(TaggedID, Serialize, Deserialize)]
pub struct DocumentID(uuid::Uuid);

pub struct Document<'a> {
    connection: &'a Connection,
    pub data: Data,
}

pub struct SerializableDocument<'a> {
    document: &'a Document<'a>,
    categories: Vec<Category<'a>>,
}

pub struct DocumentWithItems<'a> {
    document: SerializableDocument<'a>,
    items_hashmap: HashMap<ItemID, ItemWithStyles<'a>>,
}

#[derive(Debug, Queryable, Identifiable)]
#[table_name = "documents"]
pub struct Data {
    id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    root_item_id: Option<uuid::Uuid>,
    pub created_at: SystemTime,
    pub modified_at: Option<SystemTime>,
    pub publicly_viewable: bool,
    toc_item_id: Option<uuid::Uuid>,
}

#[derive(Insertable)]
#[table_name = "documents"]
struct NewDocument<'a> {
    pub user_id: &'a uuid::Uuid,
}

#[derive(Insertable)]
#[table_name = "items"]
struct NewItem<'a> {
    document_id: uuid::Uuid,
    parent_id: Option<uuid::Uuid>,
    item_text: &'a str,
    child_order: i32,
    collapsed: bool,
}

impl<'a> Document<'a> {
    pub fn get_id(&self) -> DocumentID {
        DocumentID::from_uuid(self.data.id.clone())
    }

    pub fn get_by_id(
        connection: &'a Connection,
        p_document_id: &DocumentID,
    ) -> QueryResult<Document<'a>> {
        let p_doc_uuid: uuid::Uuid = **p_document_id;

        let data = documents
            .filter(data::schema::documents::dsl::id.eq(&p_doc_uuid))
            .first::<Data>(&connection.pg_connection)?;

        Ok(Document { connection, data })
    }

    pub fn add_item(
        &mut self,
        p_parent_id: Option<ItemID>,
        p_order: i32,
        text: Option<String>,
    ) -> QueryResult<Item> {
        let insert_item = NewItem {
            document_id: *self.get_id(),
            parent_id: match p_parent_id {
                Some(pid) => Some(*pid),
                None => None,
            },
            item_text: &text.unwrap_or("".to_string()),
            child_order: p_order,
            collapsed: false,
        };

        let data = diesel::insert_into(items)
            .values(insert_item)
            .get_result(&self.connection.pg_connection)?;

        Ok(Item::new(self.connection, data))
    }

    pub fn create_for_user(
        connection: &'a Connection,
        p_user_id: &UserID,
    ) -> QueryResult<Document<'a>> {
        let p_user_uuid = **p_user_id;

        let data: Data = diesel::insert_into(documents)
            .values(NewDocument {
                user_id: &p_user_uuid,
            })
            .get_result(&connection.pg_connection)?;

        let mut doc = Document { connection, data };
        let doc_id = doc.data.id.clone();
        let root_item = doc.add_item(None, 0, None)?;

        let rooted_data = diesel::update(documents)
            .filter(data::schema::documents::dsl::id.eq(&doc_id))
            .set(root_item_id.eq(&*root_item.get_id()))
            .get_result(&connection.pg_connection)?;

        Ok(Document {
            connection,
            data: rooted_data,
        })
    }

    pub fn get_by_user(
        connection: &'a Connection,
        p_user_id: &UserID,
    ) -> QueryResult<Vec<Document<'a>>> {
        let p_user_uuid = **p_user_id;

        let docs = documents
            .filter(user_id.eq(&p_user_uuid))
            .load::<Data>(&connection.pg_connection)?;

        Ok(docs
            .into_iter()
            .map(|data| Document { connection, data })
            .collect())
    }

    pub fn touch(&mut self) -> QueryResult<&mut Document<'a>> {
        let updated = diesel::update(documents)
            .filter(data::schema::documents::dsl::id.eq(&self.data.id))
            .set(modified_at.eq(Some(std::time::SystemTime::now())))
            .get_result(&self.connection.pg_connection)?;

        self.data = updated;

        Ok(self)
    }

    pub fn can_be_viewed_by(self: &Document<'a>, p_user_id: &UserID) -> bool {
        self.can_be_viewed_anonymously() || self.data.user_id.eq(&**p_user_id)
    }

    pub fn can_be_viewed_anonymously(&self) -> bool {
        self.data.publicly_viewable
    }

    pub fn can_be_edited_by(self: &Document<'a>, p_user_id: &UserID) -> bool {
        self.data.user_id.eq(&**p_user_id)
    }

    pub fn is_owned_by(&self, p_user_id: &UserID) -> bool {
        self.data.user_id.eq(&**p_user_id)
    }

    pub fn get_items(self: &Document<'a>) -> QueryResult<Vec<super::item::Item<'a>>> {
        super::item::Item::get_by_document(self.connection, &self.get_id())
    }

    pub fn get_item(&self, p_item_id: &ItemID) -> QueryResult<Item> {
        let data = items
            .filter(document_id.eq(&*self.get_id()))
            .filter(data::schema::items::dsl::id.eq(&**p_item_id))
            .first::<super::item::Data>(&self.connection.pg_connection)?;

        Ok(Item::new(&self.connection, data))
    }

    fn copy_item_children(
        self: &mut Document<'a>,
        p_items: &Vec<super::item::Item<'a>>,
        old_parent_id: ItemID,
        new_parent_id: Option<ItemID>,
    ) -> QueryResult<()> {
        p_items
            .iter()
            .filter(|i| {
                i.get_parent_id()
                    .and_then(|pid| Some(pid == old_parent_id))
                    .unwrap_or(false)
            })
            .map(|i| {
                self.copy_item(
                    &p_items,
                    &i,
                    match new_parent_id {
                        Some(ref npi) => Some(npi.clone()),
                        None => None,
                    },
                )
            })
            .fold(Ok(()), |prev: QueryResult<()>, r| match prev {
                Err(e) => Err(e),
                Ok(()) => r,
            })
    }

    fn copy_item(
        self: &mut Document<'a>,
        p_items: &Vec<super::item::Item<'a>>,
        curr_item: &super::item::Item<'a>,
        new_parent_id: Option<ItemID>,
    ) -> QueryResult<()> {
        let new_item_id = {
            self.add_item(
                new_parent_id,
                curr_item.data.child_order,
                Some(curr_item.data.item_text.clone()),
            )?
            .get_id()
        };

        self.copy_item_children(&p_items, curr_item.get_id(), Some(new_item_id))?;

        Ok(())
    }

    pub fn copy_to_user(&self, p_user_id: &UserID) -> QueryResult<Self> {
        let mut new_document = Self::create_for_user(&self.connection, &p_user_id)?;
        new_document.rename(&self.get_title().unwrap_or("".to_string()))?;

        let new_root_id = match new_document.data.root_item_id {
            None => None,
            Some(r) => Some(ItemID::from_uuid(r.clone())),
        };

        match self.data.root_item_id {
            Some(r) => new_document.copy_item_children(
                &self.get_items()?,
                ItemID::from_uuid(r.clone()),
                new_root_id,
            ),
            None => Ok(()),
        }?;

        Ok(new_document)
    }

    pub fn get_root(&self) -> QueryResult<Item> {
        let root_id = self
            .data
            .root_item_id
            .ok_or(diesel::result::Error::NotFound)?;

        Item::get_by_id(&self.connection, &ItemID::from_uuid(root_id))
    }

    pub fn get_title(&self) -> QueryResult<String> {
        Ok(self.get_root()?.data.item_text)
    }

    pub fn rename(&mut self, update_text: &str) -> QueryResult<()> {
        let mut root_item = self.get_root()?;
        root_item.update_text(update_text)?;

        Ok(())
    }

    pub fn is_trashed(&self, p_user_id: &UserID) -> QueryResult<bool> {
        let trash_category = Category::get_category(
            &self.connection,
            &self.get_id(),
            &p_user_id,
            Category::TRASH,
        );

        match trash_category {
            Ok(_) => Ok(true),
            Err(diesel::result::Error::NotFound) => Ok(false),
            Err(e) => Err(e),
        }
    }

    fn get_serialized_title(&self) -> Option<String> {
        if let Ok(title) = self.get_title() {
            if title == "" {
                Some("Untitled Document".to_string())
            } else {
                Some(title)
            }
        } else {
            None
        }
    }

    fn get_serialized_root_id(&self) -> Option<ItemID> {
        if let Some(root_id) = self.data.root_item_id {
            Some(ItemID::from_uuid(root_id))
        } else {
            None
        }
    }

    pub fn get_serialized_toc_id(&self) -> Option<ItemID> {
        if let Some(toc_id) = self.data.toc_item_id {
            Some(ItemID::from_uuid(toc_id))
        } else {
            None
        }
    }

    pub fn delete(&mut self) -> QueryResult<usize> {
        diesel::delete(documents)
            .filter(data::schema::documents::dsl::id.eq(&self.data.id))
            .execute(&self.connection.pg_connection)
    }

    pub fn set_publicly_viewable(&mut self, p_publicly_viewable: bool) -> QueryResult<&mut Self> {
        let new_data = diesel::update(documents)
            .filter(data::schema::documents::dsl::id.eq(&self.data.id))
            .set(publicly_viewable.eq(p_publicly_viewable))
            .get_result(&self.connection.pg_connection)?;

        self.data = new_data;
        Ok(self)
    }

    pub fn set_toc_item(&mut self, p_toc_item: &Option<ItemID>) -> QueryResult<&mut Self> {
        let p_toc_item_id = p_toc_item.as_ref().and_then(|t| Some(**t));

        let new_data = diesel::update(documents)
            .filter(data::schema::documents::dsl::id.eq(&self.data.id))
            .set(toc_item_id.eq(&p_toc_item_id))
            .get_result(&self.connection.pg_connection)?;

        self.data = new_data;
        Ok(self)
    }

    pub fn serializable(
        &'a self,
        p_user_id: Option<&UserID>,
    ) -> QueryResult<SerializableDocument<'a>> {
        let categories = match p_user_id {
            Some(uid) => Category::get_categories(&self.connection, &self.get_id(), &uid)?,
            None => Vec::new(),
        };

        Ok(SerializableDocument::<'a> {
            document: &self,
            categories,
        })
    }

    pub fn serialize_with_items(
        &'a self,
        p_user_id: Option<&UserID>,
    ) -> QueryResult<DocumentWithItems<'a>> {
        let items_hashmap = self
            .get_items()?
            .into_iter()
            .map(|i| ItemWithStyles::from(i))
            .collect::<QueryResult<Vec<ItemWithStyles>>>()?
            .into_iter()
            .fold(std::collections::HashMap::new(), |mut acc, i| {
                acc.entry(i.item.get_id()).or_insert(i);
                acc
            });

        Ok(DocumentWithItems::<'a> {
            document: self.serializable(p_user_id)?,
            items_hashmap,
        })
    }
}

fn serialize_document<'a, S>(
    ser_document: &SerializableDocument<'a>,
    serializer: S,
    items_hashmap: Option<&HashMap<ItemID, ItemWithStyles>>,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let count_fields = 9;
    let ref document = ser_document.document;

    let mut serialized = serializer.serialize_struct(
        "Document",
        if items_hashmap.is_some() {
            count_fields
        } else {
            count_fields - 1
        },
    )?;

    serialized.serialize_field("document_id", &document.get_id())?;
    serialized.serialize_field("title", &document.get_serialized_title())?;
    serialized.serialize_field("root_item_id", &document.get_serialized_root_id())?;
    serialized.serialize_field("created_at", &document.data.created_at)?;
    serialized.serialize_field("modified_at", &document.data.modified_at)?;
    serialized.serialize_field("publicly_viewable", &document.data.publicly_viewable)?;
    serialized.serialize_field("toc_item_id", &document.get_serialized_toc_id())?;

    if let Some(ser_items) = items_hashmap {
        serialized.serialize_field("items", ser_items)?;
    }

    serialized.serialize_field("categories", &ser_document.categories)?;

    serialized.end()
}

impl<'a> Serialize for SerializableDocument<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_document(self, serializer, None)
    }
}

impl<'a> Serialize for DocumentWithItems<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_document(&self.document, serializer, Some(&self.items_hashmap))
    }
}
