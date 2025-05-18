// @generated automatically by Diesel CLI.

diesel::table! {
    repositories (id) {
        id -> Uuid,
        owner -> Text,
        name -> Text,
        created_at -> Timestamp,
    }
}

diesel::table! {
    stars (repository_id, stargazer) {
        repository_id -> Uuid,
        stargazer -> Text,
        starred_at -> Timestamp,
        fetched_at -> Timestamp,
    }
}

diesel::joinable!(stars -> repositories (repository_id));

diesel::allow_tables_to_appear_in_same_query!(
    repositories,
    stars,
);
