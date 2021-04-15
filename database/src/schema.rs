table! {
    ensembles (id) {
        id -> Text,
        name -> Text,
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
    }
}

table! {
    mediums (id) {
        id -> Text,
        name -> Text,
        discid -> Nullable<Text>,
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
    }
}

table! {
    recordings (id) {
        id -> Text,
        work -> Text,
        comment -> Text,
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
    work_sections (id) {
        id -> BigInt,
        work -> Text,
        title -> Text,
        before_index -> BigInt,
    }
}

table! {
    works (id) {
        id -> Text,
        composer -> Text,
        title -> Text,
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
joinable!(work_sections -> works (work));
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
    work_sections,
    works,
);
