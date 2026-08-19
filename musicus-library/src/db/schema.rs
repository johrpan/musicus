// @generated automatically by Diesel CLI.

diesel::table! {
    album_recordings (album_id, sequence_number) {
        album_id -> Text,
        recording_id -> Text,
        sequence_number -> Integer,
    }
}

diesel::table! {
    albums (album_id) {
        album_id -> Text,
        name -> Text,
        source -> Text,
        enable_updates -> Bool,
        created_at -> Timestamp,
        edited_at -> Timestamp,
        last_used_at -> Timestamp,
    }
}

diesel::table! {
    ensemble_persons (ensemble_id, sequence_number) {
        ensemble_id -> Text,
        person_id -> Text,
        instrument_id -> Nullable<Text>,
        sequence_number -> Integer,
        role_id -> Nullable<Text>,
    }
}

diesel::table! {
    ensembles (ensemble_id) {
        ensemble_id -> Text,
        name -> Text,
        source -> Text,
        enable_updates -> Bool,
        created_at -> Timestamp,
        edited_at -> Timestamp,
        last_used_at -> Timestamp,
    }
}

diesel::table! {
    instruments (instrument_id) {
        instrument_id -> Text,
        name -> Text,
        source -> Text,
        enable_updates -> Bool,
        created_at -> Timestamp,
        edited_at -> Timestamp,
        last_used_at -> Timestamp,
    }
}

diesel::table! {
    meta (id) {
        id -> Integer,
        schema_version -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    persons (person_id) {
        person_id -> Text,
        name -> Text,
        source -> Text,
        enable_updates -> Bool,
        created_at -> Timestamp,
        edited_at -> Timestamp,
        last_used_at -> Timestamp,
    }
}

diesel::table! {
    plays (play_id) {
        play_id -> Text,
        track_id -> Nullable<Text>,
        recording_id -> Text,
        played_at -> Timestamp,
    }
}

diesel::table! {
    recording_ensembles (recording_id, sequence_number) {
        recording_id -> Text,
        ensemble_id -> Text,
        role_id -> Nullable<Text>,
        sequence_number -> Integer,
    }
}

diesel::table! {
    recording_persons (recording_id, sequence_number) {
        recording_id -> Text,
        person_id -> Text,
        role_id -> Nullable<Text>,
        instrument_id -> Nullable<Text>,
        sequence_number -> Integer,
    }
}

diesel::table! {
    recording_tags (recording_id, sequence_number) {
        recording_id -> Text,
        tag_id -> Text,
        value -> Nullable<Text>,
        sequence_number -> Integer,
    }
}

diesel::table! {
    recordings (recording_id) {
        recording_id -> Text,
        work_id -> Text,
        source -> Text,
        enable_updates -> Bool,
        created_at -> Timestamp,
        edited_at -> Timestamp,
        last_used_at -> Timestamp,
    }
}

diesel::table! {
    roles (role_id) {
        role_id -> Text,
        name -> Text,
        source -> Text,
        enable_updates -> Bool,
        created_at -> Timestamp,
        edited_at -> Timestamp,
        last_used_at -> Timestamp,
    }
}

diesel::table! {
    tags (tag_id) {
        tag_id -> Text,
        name -> Text,
        takes_value -> Bool,
        source -> Text,
        enable_updates -> Bool,
        created_at -> Timestamp,
        edited_at -> Timestamp,
        last_used_at -> Timestamp,
        private -> Bool,
    }
}

diesel::table! {
    track_works (track_id, sequence_number) {
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
        path -> Text,
        created_at -> Timestamp,
        edited_at -> Timestamp,
        last_used_at -> Timestamp,
    }
}

diesel::table! {
    work_instruments (work_id, sequence_number) {
        work_id -> Text,
        instrument_id -> Text,
        sequence_number -> Integer,
    }
}

diesel::table! {
    work_persons (work_id, sequence_number) {
        work_id -> Text,
        person_id -> Text,
        role_id -> Nullable<Text>,
        sequence_number -> Integer,
    }
}

diesel::table! {
    work_tags (work_id, sequence_number) {
        work_id -> Text,
        tag_id -> Text,
        value -> Nullable<Text>,
        sequence_number -> Integer,
    }
}

diesel::table! {
    works (work_id) {
        work_id -> Text,
        parent_work_id -> Nullable<Text>,
        sequence_number -> Nullable<Integer>,
        name -> Text,
        source -> Text,
        enable_updates -> Bool,
        created_at -> Timestamp,
        edited_at -> Timestamp,
        last_used_at -> Timestamp,
        relates_to -> Nullable<Text>,
    }
}

diesel::joinable!(album_recordings -> albums (album_id));
diesel::joinable!(album_recordings -> recordings (recording_id));
diesel::joinable!(ensemble_persons -> ensembles (ensemble_id));
diesel::joinable!(ensemble_persons -> instruments (instrument_id));
diesel::joinable!(ensemble_persons -> persons (person_id));
diesel::joinable!(ensemble_persons -> roles (role_id));
diesel::joinable!(plays -> recordings (recording_id));
diesel::joinable!(plays -> tracks (track_id));
diesel::joinable!(recording_ensembles -> ensembles (ensemble_id));
diesel::joinable!(recording_ensembles -> recordings (recording_id));
diesel::joinable!(recording_ensembles -> roles (role_id));
diesel::joinable!(recording_persons -> instruments (instrument_id));
diesel::joinable!(recording_persons -> persons (person_id));
diesel::joinable!(recording_persons -> recordings (recording_id));
diesel::joinable!(recording_persons -> roles (role_id));
diesel::joinable!(recording_tags -> recordings (recording_id));
diesel::joinable!(recording_tags -> tags (tag_id));
diesel::joinable!(recordings -> works (work_id));
diesel::joinable!(track_works -> tracks (track_id));
diesel::joinable!(track_works -> works (work_id));
diesel::joinable!(tracks -> recordings (recording_id));
diesel::joinable!(work_instruments -> instruments (instrument_id));
diesel::joinable!(work_instruments -> works (work_id));
diesel::joinable!(work_persons -> persons (person_id));
diesel::joinable!(work_persons -> roles (role_id));
diesel::joinable!(work_persons -> works (work_id));
diesel::joinable!(work_tags -> tags (tag_id));
diesel::joinable!(work_tags -> works (work_id));

diesel::allow_tables_to_appear_in_same_query!(
    album_recordings,
    albums,
    ensemble_persons,
    ensembles,
    instruments,
    meta,
    persons,
    plays,
    recording_ensembles,
    recording_persons,
    recording_tags,
    recordings,
    roles,
    tags,
    track_works,
    tracks,
    work_instruments,
    work_persons,
    work_tags,
    works,
);
