use diesel::prelude::*;

use data::db::Connection;
use data::item::ItemID;
use data::schema::styles;
use data::schema::styles::dsl::*;
use data::schema::{StyleProperty, StyleUnit};

use serde::ser::{Serialize, SerializeStruct, Serializer};

use std::collections::HashMap;

pub struct Style<'a> {
    connection: &'a Connection,
    pub data: Data,
}

#[derive(Insertable, AsChangeset, Queryable)]
#[table_name = "styles"]
#[primary_key(item_id, property)]
pub struct Data {
    item_id: uuid::Uuid,
    pub property: StyleProperty,
    pub value_number: Option<i32>,
    pub value_string: Option<String>,
    pub unit: Option<StyleUnit>,
}

impl<'a> Style<'a> {
    pub fn new(connection: &'a Connection, data: Data) -> Style<'a> {
        Style { connection, data }
    }

    pub fn create(
        connection: &'a Connection,
        p_item_id: &ItemID,
        p_property: StyleProperty,
        p_value_string: Option<String>,
        p_value_number: Option<i32>,
        p_unit: Option<StyleUnit>,
    ) -> Style<'a> {
        Style {
            connection,
            data: Data {
                item_id: (*p_item_id).clone(),
                property: p_property,
                value_string: p_value_string,
                value_number: p_value_number,
                unit: p_unit,
            },
        }
    }

    pub fn update(&mut self) -> QueryResult<&mut Self> {
        let data = diesel::update(styles)
            .filter(item_id.eq(&self.data.item_id))
            .filter(property.eq(self.data.property))
            .set((
                value_string.eq(&self.data.value_string),
                value_number.eq(&self.data.value_number),
                unit.eq(&self.data.unit),
            ))
            .get_result(&self.connection.pg_connection)?;

        self.data = data;
        Ok(self)
    }

    pub fn get_by_item(
        connection: &'a Connection,
        p_item_id: &ItemID,
    ) -> QueryResult<Vec<Style<'a>>> {
        let p_item_uuid = **p_item_id;

        let styles_list = styles
            .filter(item_id.eq(&p_item_uuid))
            .load::<Data>(&connection.pg_connection)?;

        Ok(styles_list
            .into_iter()
            .map(|data| Style { connection, data })
            .collect())
    }
}

impl<'a> Serialize for Style<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serialized = serializer.serialize_struct("Style", 4)?;

        serialized.serialize_field("property", &self.data.property.to_string())?;
        serialized.serialize_field("value_string", &self.data.value_string)?;
        serialized.serialize_field("value_number", &self.data.value_number)?;
        serialized.serialize_field("unit", &self.data.unit)?;

        serialized.end()
    }
}

pub fn style_vec_to_map<'a>(style_vec: Vec<Style<'a>>) -> HashMap<StyleProperty, Style<'a>> {
    style_vec
        .into_iter()
        .map(|se| (se.data.property, se))
        .collect()
}
