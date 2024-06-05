// @generated automatically by Diesel CLI.

diesel::table! {
    album_mediums (album_id, medium_id) {
        album_id -> Text,
        medium_id -> Text,
        sequence_number -> Integer,
    }
}

diesel::table! {
    album_recordings (album_id, recording_id) {
        album_id -> Text,
        recording_id -> Text,
        sequence_number -> Integer,
    }
}

diesel::table! {
    albums (album_id) {
        album_id -> Text,
        name -> Text,
        created_at -> Timestamp,
        edited_at -> Timestamp,
        last_used_at -> Timestamp,
        last_played_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    ensemble_persons (ensemble_id, person_id, instrument_id) {
        ensemble_id -> Text,
        person_id -> Text,
        instrument_id -> Text,
        sequence_number -> Integer,
    }
}

diesel::table! {
    ensembles (ensemble_id) {
        ensemble_id -> Text,
        name -> Text,
        created_at -> Timestamp,
        edited_at -> Timestamp,
        last_used_at -> Timestamp,
        last_played_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    instruments (instrument_id) {
        instrument_id -> Text,
        name -> Text,
        created_at -> Timestamp,
        edited_at -> Timestamp,
        last_used_at -> Timestamp,
        last_played_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    mediums (medium_id) {
        medium_id -> Text,
        discid -> Text,
        created_at -> Timestamp,
        edited_at -> Timestamp,
        last_used_at -> Timestamp,
        last_played_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    persons (person_id) {
        person_id -> Text,
        name -> Text,
        created_at -> Timestamp,
        edited_at -> Timestamp,
        last_used_at -> Timestamp,
        last_played_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    recording_ensembles (recording_id, ensemble_id, role_id) {
        recording_id -> Text,
        ensemble_id -> Text,
        role_id -> Text,
        sequence_number -> Integer,
    }
}

diesel::table! {
    recording_persons (recording_id, person_id, role_id, instrument_id) {
        recording_id -> Text,
        person_id -> Text,
        role_id -> Text,
        instrument_id -> Nullable<Text>,
        sequence_number -> Integer,
    }
}

diesel::table! {
    recordings (recording_id) {
        recording_id -> Text,
        work_id -> Text,
        year -> Nullable<Integer>,
        created_at -> Timestamp,
        edited_at -> Timestamp,
        last_used_at -> Timestamp,
        last_played_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    roles (role_id) {
        role_id -> Text,
        name -> Text,
        created_at -> Timestamp,
        edited_at -> Timestamp,
        last_used_at -> Timestamp,
    }
}

diesel::table! {
    track_works (track_id, work_id) {
        track_id -> Text,
        work_id -> Text,
        sequence_number -> Integer,
    }
}

diesel::table! {
    tracks (track_id) {
        track_id -> Text,
        recording_id -> Text,
        recording_index -> Integer,
        medium_id -> Nullable<Text>,
        medium_index -> Nullable<Integer>,
        path -> Text,
        created_at -> Timestamp,
        edited_at -> Timestamp,
        last_used_at -> Timestamp,
        last_played_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    work_instruments (work_id, instrument_id) {
        work_id -> Text,
        instrument_id -> Text,
        sequence_number -> Integer,
    }
}

diesel::table! {
    work_persons (work_id, person_id, role_id) {
        work_id -> Text,
        person_id -> Text,
        role_id -> Text,
        sequence_number -> Integer,
    }
}

diesel::table! {
    work_sections (id) {
        id -> BigInt,
        work -> Text,
        title -> Text,
        before_index -> BigInt,
    }
}

diesel::table! {
    works (work_id) {
        work_id -> Text,
        parent_work_id -> Nullable<Text>,
        sequence_number -> Nullable<Integer>,
        name -> Text,
        created_at -> Timestamp,
        edited_at -> Timestamp,
        last_used_at -> Timestamp,
        last_played_at -> Nullable<Timestamp>,
    }
}

diesel::joinable!(album_mediums -> albums (album_id));
diesel::joinable!(album_mediums -> mediums (medium_id));
diesel::joinable!(album_recordings -> albums (album_id));
diesel::joinable!(album_recordings -> recordings (recording_id));
diesel::joinable!(ensemble_persons -> ensembles (ensemble_id));
diesel::joinable!(ensemble_persons -> instruments (instrument_id));
diesel::joinable!(ensemble_persons -> persons (person_id));
diesel::joinable!(recording_ensembles -> ensembles (ensemble_id));
diesel::joinable!(recording_ensembles -> recordings (recording_id));
diesel::joinable!(recording_ensembles -> roles (role_id));
diesel::joinable!(recording_persons -> instruments (instrument_id));
diesel::joinable!(recording_persons -> persons (person_id));
diesel::joinable!(recording_persons -> recordings (recording_id));
diesel::joinable!(recording_persons -> roles (role_id));
diesel::joinable!(recordings -> works (work_id));
diesel::joinable!(track_works -> tracks (track_id));
diesel::joinable!(track_works -> works (work_id));
diesel::joinable!(tracks -> mediums (medium_id));
diesel::joinable!(tracks -> recordings (recording_id));
diesel::joinable!(work_instruments -> instruments (instrument_id));
diesel::joinable!(work_instruments -> works (work_id));
diesel::joinable!(work_persons -> persons (person_id));
diesel::joinable!(work_persons -> roles (role_id));
diesel::joinable!(work_persons -> works (work_id));

diesel::allow_tables_to_appear_in_same_query!(
    album_mediums,
    album_recordings,
    albums,
    ensemble_persons,
    ensembles,
    instruments,
    mediums,
    persons,
    recording_ensembles,
    recording_persons,
    recordings,
    roles,
    track_works,
    tracks,
    work_instruments,
    work_persons,
    work_sections,
    works,
);
