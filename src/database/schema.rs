table! {
    ensembles (id) {
        id -> BigInt,
        name -> Text,
    }
}

table! {
    instrumentations (id) {
        id -> BigInt,
        work -> BigInt,
        instrument -> BigInt,
    }
}

table! {
    instruments (id) {
        id -> BigInt,
        name -> Text,
    }
}

table! {
    part_instrumentations (id) {
        id -> BigInt,
        work_part -> BigInt,
        instrument -> BigInt,
    }
}

table! {
    performances (id) {
        id -> BigInt,
        recording -> BigInt,
        person -> Nullable<BigInt>,
        ensemble -> Nullable<BigInt>,
        role -> Nullable<BigInt>,
    }
}

table! {
    persons (id) {
        id -> BigInt,
        first_name -> Text,
        last_name -> Text,
    }
}

table! {
    recordings (id) {
        id -> BigInt,
        work -> BigInt,
        comment -> Text,
    }
}

table! {
    tracks (id) {
        id -> BigInt,
        file_name -> Text,
        recording -> BigInt,
        track_index -> Integer,
        work_parts -> Text,
    }
}

table! {
    work_parts (id) {
        id -> BigInt,
        work -> BigInt,
        part_index -> BigInt,
        composer -> Nullable<BigInt>,
        title -> Text,
    }
}

table! {
    work_sections (id) {
        id -> BigInt,
        work -> BigInt,
        title -> Text,
        before_index -> BigInt,
    }
}

table! {
    works (id) {
        id -> BigInt,
        composer -> BigInt,
        title -> Text,
    }
}

joinable!(instrumentations -> instruments (instrument));
joinable!(instrumentations -> works (work));
joinable!(part_instrumentations -> instruments (instrument));
joinable!(part_instrumentations -> works (work_part));
joinable!(performances -> ensembles (ensemble));
joinable!(performances -> instruments (role));
joinable!(performances -> persons (person));
joinable!(performances -> recordings (recording));
joinable!(recordings -> works (work));
joinable!(tracks -> recordings (recording));
joinable!(work_parts -> persons (composer));
joinable!(work_parts -> works (work));
joinable!(work_sections -> works (work));
joinable!(works -> persons (composer));

allow_tables_to_appear_in_same_query!(
    ensembles,
    instrumentations,
    instruments,
    part_instrumentations,
    performances,
    persons,
    recordings,
    tracks,
    work_parts,
    work_sections,
    works,
);
