// @generated automatically by Diesel CLI.

diesel::table! {
    master_table (id) {
        id -> Integer,
        encrypted_master_key -> Binary,
        nonce -> Binary,
        salt -> Binary,
    }
}

diesel::table! {
    records (id) {
        id -> Integer,
        service -> Text,
        email -> Text,
        username -> Text,
        password -> Text,
        notes -> Text,
    }
}

diesel::allow_tables_to_appear_in_same_query!(master_table, records,);
