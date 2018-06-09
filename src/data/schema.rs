table! {
    documents (document_id) {
        document_id -> Uuid,
        user_id -> Uuid,
        root_item_id -> Nullable<Uuid>,
        created_at -> Timestamp,
        modified_at -> Nullable<Timestamp>,
    }
}

table! {
    items (item_id) {
        item_id -> Uuid,
        document_id -> Uuid,
        parent_id -> Nullable<Uuid>,
        item_text -> Text,
        collapsed -> Nullable<Bool>,
    }
}

table! {
    users (user_id) {
        user_id -> Uuid,
        twitter_name -> Nullable<Text>,
        twitter_screen_name -> Nullable<Text>,
    }
}

allow_tables_to_appear_in_same_query!(documents, items, users,);
