table! {
    records (id) {
        id -> Int4,
        title -> Nullable<Text>,
        source_record_id -> Text,
        source_id -> Int4,
        content -> Text,
        date -> Timestamp,
        image -> Nullable<Text>,
        external_link -> Text,
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
        external_link -> Text,
    }
}

joinable!(records -> sources (source_id));

allow_tables_to_appear_in_same_query!(records, sources,);
