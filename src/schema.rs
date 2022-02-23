table! {
    events (id) {
        id -> Integer,
        timestamp -> Text,
        event -> Binary,
    }
}

table! {
    staff (id) {
        id -> Integer,
        name -> Text,
        pin -> Text,
        cardid -> Text,
        status -> Bool,
    }
}

allow_tables_to_appear_in_same_query!(
    events,
    staff,
);
