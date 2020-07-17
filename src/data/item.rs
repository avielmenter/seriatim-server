use diesel;
use diesel::prelude::*;

use data;
use data::db::Connection;
use data::document::DocumentID;
use data::schema::items;
use data::schema::items::dsl::*;
use data::schema::styles::dsl::*;
use data::schema::{StyleProperty, StyleUnit};
use data::style::{style_vec_to_map, Style};

use serde::ser::{Serialize, SerializeStruct, Serializer};

use std::collections::HashMap;

use uuid;

#[derive(TaggedID, Serialize, Deserialize)]
pub struct ItemID(uuid::Uuid);

pub struct Item<'a> {
    connection: &'a Connection,
    pub data: Data,
}

#[derive(Identifiable, AsChangeset, Queryable)]
#[table_name = "items"]
pub struct Data {
    id: uuid::Uuid,
    pub document_id: uuid::Uuid,
    pub parent_id: Option<uuid::Uuid>,
    pub item_text: String,
    pub child_order: i32,
    pub collapsed: bool,
}

impl<'a> Item<'a> {
    pub fn new(connection: &'a Connection, data: Data) -> Item<'a> {
        Item { connection, data }
    }

    pub fn get_id(&self) -> ItemID {
        ItemID::from_uuid(self.data.id.clone())
    }

    pub fn get_parent_id(&self) -> Option<ItemID> {
        self.data
            .parent_id
            .and_then(|p| Some(ItemID::from_uuid(p.clone())))
    }

    fn results_list(
        connection: &'a Connection,
        items_list: Vec<Data>,
    ) -> QueryResult<Vec<Item<'a>>> {
        Ok(items_list
            .into_iter()
            .map(|data| Item { connection, data })
            .collect())
    }

    pub fn get_by_id(connection: &'a Connection, p_item_id: &ItemID) -> QueryResult<Item<'a>> {
        let p_uuid = **p_item_id;

        let data = items
            .filter(id.eq(&p_uuid))
            .first::<Data>(&connection.pg_connection)?;

        Ok(Item { connection, data })
    }

    pub fn get_by_document(
        connection: &'a Connection,
        p_document_id: &DocumentID,
    ) -> QueryResult<Vec<Item<'a>>> {
        let p_doc_uuid = **p_document_id;

        let items_list = items
            .filter(document_id.eq(&p_doc_uuid))
            .load::<Data>(&connection.pg_connection)?;

        Self::results_list(connection, items_list)
    }

    pub fn remove_children(&mut self) -> QueryResult<usize> {
        let p_item_uuid = self.get_id();

        diesel::delete(items)
            .filter(parent_id.eq(&*p_item_uuid))
            .execute(&self.connection.pg_connection)
    }

    pub fn create_style(
        &self,
        p_property: StyleProperty,
        p_value_string: Option<String>,
        p_value_number: Option<i32>,
        p_unit: Option<StyleUnit>,
    ) -> Style<'a> {
        Style::create(
            self.connection,
            &self.get_id(),
            p_property,
            p_value_string,
            p_value_number,
            p_unit,
        )
    }

    pub fn update_text(&mut self, update_text: &str) -> QueryResult<&mut Item<'a>> {
        let data = diesel::update(items)
            .filter(id.eq(self.data.id))
            .set(item_text.eq(update_text))
            .get_result(&self.connection.pg_connection)?;

        self.data = data;

        Ok(self)
    }

    fn add_style(&mut self, style: &Style<'a>) -> QueryResult<Style<'a>> {
        let data = diesel::insert_into(styles)
            .values(&style.data)
            .get_result(&self.connection.pg_connection)?;

        Ok(Style::new(self.connection, data))
    }

    pub fn update_styles(&mut self, update_styles: Vec<Style<'a>>) -> QueryResult<Vec<Style<'a>>> {
        let current_properties: Vec<data::schema::StyleProperty> =
            Style::get_by_item(&self.connection, &self.get_id())?
                .into_iter()
                .map(|s| s.data.property)
                .collect();

        update_styles
            .into_iter()
            .map(|mut se| {
                if current_properties.contains(&se.data.property) {
                    se.update()?;
                    Ok(se)
                } else {
                    self.add_style(&se)
                }
            })
            .collect()
    }
}

fn serialize_item<'a, S>(
    item: &Item<'a>,
    serializer: S,
    styles_map: Option<&HashMap<StyleProperty, Style<'a>>>,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut serialized =
        serializer.serialize_struct("Item", if styles_map.is_some() { 7 } else { 6 })?;
    serialized.serialize_field("item_id", &item.get_id())?;

    serialized.serialize_field(
        "document_id",
        &item.data.document_id.hyphenated().to_string(),
    )?;

    serialized.serialize_field(
        "parent_id",
        &item
            .data
            .parent_id
            .and_then(|p| Some(p.hyphenated().to_string())),
    )?;

    serialized.serialize_field("text", &item.data.item_text)?;
    serialized.serialize_field("child_order", &item.data.child_order)?;
    serialized.serialize_field("collapsed", &item.data.collapsed)?;

    if let Some(s) = styles_map {
        serialized.serialize_field("styles", s)?;
    }

    serialized.end()
}

impl<'a> Serialize for Item<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_item(self, serializer, None)
    }
}

pub struct ItemWithStyles<'a> {
    pub item: Item<'a>,
    pub styles: HashMap<StyleProperty, Style<'a>>,
}

impl<'a> ItemWithStyles<'a> {
    pub fn from(item: Item<'a>) -> QueryResult<ItemWithStyles<'a>> {
        let item_styles = Style::get_by_item(&item.connection, &(item.get_id()))?;

        Ok(ItemWithStyles::<'a> {
            item: item,
            styles: style_vec_to_map(item_styles),
        })
    }
}

impl<'a> Serialize for ItemWithStyles<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_item(&self.item, serializer, Some(&self.styles))
    }
}
