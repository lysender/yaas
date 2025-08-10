// @generated automatically by Diesel CLI.

diesel::table! {
    apps (id) {
        #[max_length = 36]
        id -> Bpchar,
        #[max_length = 100]
        name -> Varchar,
        #[max_length = 200]
        secret -> Varchar,
        #[max_length = 250]
        redirect_uri -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    oauth_codes (id) {
        #[max_length = 36]
        id -> Bpchar,
        #[max_length = 36]
        code -> Bpchar,
        #[max_length = 250]
        state -> Varchar,
        #[max_length = 250]
        redirect_uri -> Varchar,
        #[max_length = 250]
        scope -> Varchar,
        #[max_length = 36]
        app_id -> Bpchar,
        #[max_length = 36]
        org_id -> Bpchar,
        #[max_length = 36]
        user_id -> Bpchar,
        created_at -> Timestamptz,
        expires_at -> Timestamptz,
    }
}

diesel::table! {
    org_apps (id) {
        #[max_length = 36]
        id -> Bpchar,
        #[max_length = 36]
        org_id -> Bpchar,
        #[max_length = 36]
        app_id -> Bpchar,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    org_members (id) {
        #[max_length = 36]
        id -> Bpchar,
        #[max_length = 36]
        org_id -> Bpchar,
        #[max_length = 36]
        user_id -> Bpchar,
        #[max_length = 250]
        roles -> Varchar,
        #[max_length = 10]
        status -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    orgs (id) {
        #[max_length = 36]
        id -> Bpchar,
        #[max_length = 100]
        name -> Varchar,
        #[max_length = 10]
        status -> Varchar,
        #[max_length = 36]
        owner_id -> Bpchar,
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
    users,
);
