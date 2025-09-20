// @generated automatically by Diesel CLI.

diesel::table! {
    apps (id) {
        id -> Int4,
        #[max_length = 100]
        name -> Varchar,
        #[max_length = 36]
        client_id -> Varchar,
        #[max_length = 200]
        client_secret -> Varchar,
        #[max_length = 250]
        redirect_uri -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        deleted_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    oauth_codes (id) {
        id -> Int4,
        #[max_length = 36]
        code -> Bpchar,
        #[max_length = 250]
        state -> Varchar,
        #[max_length = 250]
        redirect_uri -> Varchar,
        #[max_length = 250]
        scope -> Varchar,
        app_id -> Int4,
        org_id -> Int4,
        user_id -> Int4,
        created_at -> Timestamptz,
        expires_at -> Timestamptz,
    }
}

diesel::table! {
    org_apps (id) {
        id -> Int4,
        org_id -> Int4,
        app_id -> Int4,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    org_members (id) {
        id -> Int4,
        org_id -> Int4,
        user_id -> Int4,
        #[max_length = 255]
        roles -> Varchar,
        #[max_length = 10]
        status -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    orgs (id) {
        id -> Int4,
        #[max_length = 100]
        name -> Varchar,
        #[max_length = 10]
        status -> Varchar,
        owner_id -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        deleted_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    passwords (id) {
        id -> Int4,
        #[max_length = 250]
        password -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    superusers (id) {
        id -> Int4,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        #[max_length = 255]
        email -> Varchar,
        #[max_length = 100]
        name -> Varchar,
        #[max_length = 10]
        status -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        deleted_at -> Nullable<Timestamptz>,
    }
}

diesel::joinable!(oauth_codes -> apps (app_id));
diesel::joinable!(oauth_codes -> orgs (org_id));
diesel::joinable!(oauth_codes -> users (user_id));
diesel::joinable!(org_apps -> apps (app_id));
diesel::joinable!(org_apps -> orgs (org_id));
diesel::joinable!(org_members -> orgs (org_id));
diesel::joinable!(org_members -> users (user_id));
diesel::joinable!(orgs -> users (owner_id));

diesel::allow_tables_to_appear_in_same_query!(
    apps,
    oauth_codes,
    org_apps,
    org_members,
    orgs,
    passwords,
    superusers,
    users,
);
