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
        pin -> Text,
        cardid -> Text,
    }
}

allow_tables_to_appear_in_same_query!(
    events,
    passwords,
    staff,
);
