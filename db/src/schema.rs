// @generated automatically by Diesel CLI.

diesel::table! {
    orgs (id) {
        #[max_length = 36]
        id -> Bpchar,
        #[max_length = 100]
        name -> Varchar,
        #[max_length = 10]
        status -> Varchar,
        #[max_length = 36]
        owner_id -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    passwords (id) {
        #[max_length = 36]
        id -> Bpchar,
        #[max_length = 250]
        password -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

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

diesel::joinable!(orgs -> users (owner_id));

diesel::allow_tables_to_appear_in_same_query!(
    orgs,
    passwords,
    users,
);
