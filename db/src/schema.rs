// @generated automatically by Diesel CLI.

diesel::table! {
    users (id) {
        #[max_length = 36]
        id -> Bpchar,
        #[max_length = 250]
        email -> Varchar,
        #[max_length = 100]
        name -> Varchar,
        #[max_length = 10]
        status -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}
