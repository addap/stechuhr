table! {
    events (id) {
        id -> Integer,
        created_at -> Timestamp,
        event_json -> Text,
    }
}

table! {
    passwords (id) {
        id -> Integer,
        phc -> Text,
    }
}

table! {
    staff (id) {
        id -> Integer,
        name -> Text,
        pin -> Nullable<Text>,
        cardid -> Nullable<Text>,
        is_visible -> Bool,
        is_active -> Bool,
    }
}

allow_tables_to_appear_in_same_query!(events, passwords, staff,);
