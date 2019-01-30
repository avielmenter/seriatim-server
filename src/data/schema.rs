use std::string::ToString;

#[derive(Debug, DbEnum, PartialEq, Eq, Hash, Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StyleProperty {
    BackgroundColor,
    Color,
    FontSize,
    LineHeight,
}

impl ToString for StyleProperty {
    fn to_string(&self) -> String {
        match self {
            StyleProperty::BackgroundColor => String::from("backgroundColor"),
            StyleProperty::Color => String::from("color"),
            StyleProperty::FontSize => String::from("fontSize"),
            StyleProperty::LineHeight => String::from("lineHeight"),
        }
    }
}

#[derive(Debug, DbEnum, PartialEq, Eq, Hash, Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StyleUnit {
    Cm,
    Mm,
    In,
    Px,
    Pt,
    Pc,
    Em,
    Ex,
    Ch,
    Rem,
    Vw,
    Vh,
    Vmin,
    Vmax,
    #[db_rename = "%"]
    Ppct,
}

impl ToString for StyleUnit {
    #[allow(unused_variables)]
    fn to_string(&self) -> String {
        match self {
            StyleUnit::Cm => String::from("cm"),
            StyleUnit::Mm => String::from("mm"),
            StyleUnit::In => String::from("in"),
            StyleUnit::Px => String::from("px"),
            StyleUnit::Pt => String::from("pt"),
            StyleUnit::Pc => String::from("pc"),
            StyleUnit::Em => String::from("em"),
            StyleUnit::Ex => String::from("ex"),
            StyleUnit::Ch => String::from("ch"),
            StyleUnit::Rem => String::from("rem"),
            StyleUnit::Vw => String::from("vw"),
            StyleUnit::Vh => String::from("vh"),
            StyleUnit::Vmin => String::from("vmin"),
            StyleUnit::Vmax => String::from("vmax"),
            StyleUnit::Ppct => String::from("%"),
        }
    }
}

table! {
    categories (id) {
        id -> Uuid,
        user_id -> Uuid,
        document_id -> Uuid,
        category_name -> Text,
    }
}

table! {
    documents (id) {
        id -> Uuid,
        user_id -> Uuid,
        root_item_id -> Nullable<Uuid>,
        created_at -> Timestamp,
        modified_at -> Nullable<Timestamp>,
        publicly_viewable -> Bool,
        toc_item_id -> Nullable<Uuid>,
    }
}

table! {
    items (id) {
        id -> Uuid,
        document_id -> Uuid,
        parent_id -> Nullable<Uuid>,
        item_text -> Text,
        child_order -> Int4,
        collapsed -> Bool,
    }
}

table! {
    use diesel::sql_types::{Uuid, Nullable, Int4, Text};
    use super::StylePropertyMapping;
    use super::StyleUnitMapping;
    styles (item_id, property) {
        item_id -> Uuid,
        property -> StylePropertyMapping,
        value_number -> Nullable<Int4>,
        value_string -> Nullable<Text>,
        unit -> Nullable<StyleUnitMapping>,
    }
}

table! {
    users (id) {
        id -> Uuid,
        display_name -> Text,
        google_id -> Nullable<Text>,
        twitter_screen_name -> Nullable<Text>,
        facebook_id -> Nullable<Text>,
    }
}

joinable!(categories -> documents (document_id));
joinable!(categories -> users (user_id));
joinable!(documents -> users (user_id));
joinable!(styles -> items (item_id));

allow_tables_to_appear_in_same_query!(categories, documents, items, users, styles);
