// @generated automatically by Diesel CLI.

diesel::table! {
    ensembles (id) {
        id -> Text,
        name -> Text,
        last_used -> Nullable<BigInt>,
        last_played -> Nullable<BigInt>,
    }
}

diesel::table! {
    instrumentations (id) {
        id -> BigInt,
        work -> Text,
        instrument -> Text,
    }
}

diesel::table! {
    instruments (id) {
        id -> Text,
        name -> Text,
        last_used -> Nullable<BigInt>,
        last_played -> Nullable<BigInt>,
    }
}

diesel::table! {
    mediums (id) {
        id -> Text,
        name -> Text,
        discid -> Nullable<Text>,
        last_used -> Nullable<BigInt>,
        last_played -> Nullable<BigInt>,
    }
}

diesel::table! {
    performances (id) {
        id -> BigInt,
        recording -> Text,
        person -> Nullable<Text>,
        ensemble -> Nullable<Text>,
        role -> Nullable<Text>,
    }
}

diesel::table! {
    persons (id) {
        id -> Text,
        first_name -> Text,
        last_name -> Text,
        last_used -> Nullable<BigInt>,
        last_played -> Nullable<BigInt>,
    }
}

diesel::table! {
    recordings (id) {
        id -> Text,
        work -> Text,
        comment -> Text,
        last_used -> Nullable<BigInt>,
        last_played -> Nullable<BigInt>,
    }
}

diesel::table! {
    tracks (id) {
        id -> Text,
        medium -> Nullable<Text>,
        index -> Integer,
        recording -> Text,
        work_parts -> Text,
        source_index -> Integer,
        path -> Text,
        last_used -> Nullable<BigInt>,
        last_played -> Nullable<BigInt>,
    }
}

diesel::table! {
    work_parts (id) {
        id -> BigInt,
        work -> Text,
        part_index -> BigInt,
        title -> Text,
    }
}

diesel::table! {
    works (id) {
        id -> Text,
        composer -> Text,
        title -> Text,
        last_used -> Nullable<BigInt>,
        last_played -> Nullable<BigInt>,
    }
}

diesel::joinable!(instrumentations -> instruments (instrument));
diesel::joinable!(instrumentations -> works (work));
diesel::joinable!(performances -> ensembles (ensemble));
diesel::joinable!(performances -> instruments (role));
diesel::joinable!(performances -> persons (person));
diesel::joinable!(performances -> recordings (recording));
diesel::joinable!(recordings -> works (work));
diesel::joinable!(tracks -> mediums (medium));
diesel::joinable!(tracks -> recordings (recording));
diesel::joinable!(work_parts -> works (work));
diesel::joinable!(works -> persons (composer));

diesel::allow_tables_to_appear_in_same_query!(
    ensembles,
    instrumentations,
    instruments,
    mediums,
    performances,
    persons,
    recordings,
    tracks,
    work_parts,
    works,
);
