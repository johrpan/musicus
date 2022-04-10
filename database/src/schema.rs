table! {
    ensembles (id) {
        id -> Text,
        name -> Text,
        last_used -> Nullable<BigInt>,
        last_played -> Nullable<BigInt>,
    }
}

table! {
    instrumentations (id) {
        id -> BigInt,
        work -> Text,
        instrument -> Text,
    }
}

table! {
    instruments (id) {
        id -> Text,
        name -> Text,
        last_used -> Nullable<BigInt>,
        last_played -> Nullable<BigInt>,
    }
}

table! {
    mediums (id) {
        id -> Text,
        name -> Text,
        discid -> Nullable<Text>,
        last_used -> Nullable<BigInt>,
        last_played -> Nullable<BigInt>,
    }
}

table! {
    performances (id) {
        id -> BigInt,
        recording -> Text,
        person -> Nullable<Text>,
        ensemble -> Nullable<Text>,
        role -> Nullable<Text>,
    }
}

table! {
    persons (id) {
        id -> Text,
        first_name -> Text,
        last_name -> Text,
        last_used -> Nullable<BigInt>,
        last_played -> Nullable<BigInt>,
    }
}

table! {
    recordings (id) {
        id -> Text,
        work -> Text,
        comment -> Text,
        last_used -> Nullable<BigInt>,
        last_played -> Nullable<BigInt>,
    }
}

table! {
    tracks (id) {
        id -> Text,
        medium -> Text,
        index -> Integer,
        recording -> Text,
        work_parts -> Text,
        source_index -> Integer,
        path -> Text,
        last_used -> Nullable<BigInt>,
        last_played -> Nullable<BigInt>,
    }
}

table! {
    work_parts (id) {
        id -> BigInt,
        work -> Text,
        part_index -> BigInt,
        title -> Text,
    }
}

table! {
    works (id) {
        id -> Text,
        composer -> Text,
        title -> Text,
        last_used -> Nullable<BigInt>,
        last_played -> Nullable<BigInt>,
    }
}

joinable!(instrumentations -> instruments (instrument));
joinable!(instrumentations -> works (work));
joinable!(performances -> ensembles (ensemble));
joinable!(performances -> instruments (role));
joinable!(performances -> persons (person));
joinable!(performances -> recordings (recording));
joinable!(recordings -> works (work));
joinable!(tracks -> mediums (medium));
joinable!(tracks -> recordings (recording));
joinable!(work_parts -> works (work));
joinable!(works -> persons (composer));

allow_tables_to_appear_in_same_query!(
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
