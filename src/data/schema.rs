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

allow_tables_to_appear_in_same_query!(categories, documents, items, users,);
