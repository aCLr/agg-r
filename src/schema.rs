table! {
    records (id) {
        id -> Int4,
        title -> Nullable<Text>,
        guid -> Text,
        source_id -> Int4,
        content -> Text,
        date -> Timestamp,
        image -> Nullable<Text>,
    }
}

table! {
    records_meta (id) {
        id -> Int4,
        user_id -> Int4,
        record_id -> Int4,
        starred -> Bool,
    }
}

table! {
    settings (id) {
        id -> Int4,
        user_id -> Int4,
        name -> Nullable<Text>,
        value -> Nullable<Text>,
    }
}

table! {
    sources (id) {
        id -> Int4,
        name -> Text,
        origin -> Text,
        kind -> Text,
        image -> Nullable<Text>,
        last_scrape_time -> Timestamp,
    }
}

table! {
    users (id) {
        id -> Int4,
        last_read_date -> Timestamp,
    }
}

joinable!(records -> sources (source_id));
joinable!(records_meta -> records (record_id));
joinable!(records_meta -> users (user_id));

allow_tables_to_appear_in_same_query!(records, records_meta, settings, sources, users,);
