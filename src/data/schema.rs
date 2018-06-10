table! {
    documents (id) {
        id -> Uuid,
        user_id -> Uuid,
        root_item_id -> Nullable<Uuid>,
        created_at -> Timestamp,
        modified_at -> Nullable<Timestamp>,
    }
}

table! {
    items (id) {
        id -> Uuid,
        document_id -> Uuid,
        parent_id -> Nullable<Uuid>,
        item_text -> Text,
        collapsed -> Bool,
    }
}

table! {
    users (id) {
        id -> Uuid,
        twitter_name -> Nullable<Text>,
        twitter_screen_name -> Nullable<Text>,
    }
}

allow_tables_to_appear_in_same_query!(documents, items, users,);
