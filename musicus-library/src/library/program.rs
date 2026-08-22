//! Listening history and program generation: recording that a track was
//! played, and using that history to choose what to play next.
//!
//! Track selection is based on settings within a program. The main concept is
//! based an exponential function of the form:
//!
//! `exp(STRENGTH * preference * (score - 1))`
//!
//! with `STRENGTH` being a hard coded constant for tweaking the association,
//! `preference` being a user supplied value in the range `[0; 1]` and `score`
//! being the property of the recording that is being evaluated.
//!
//! Additionally, there is a linear decay for avoiding repeated entities. Both
//! are combined to give each recording a weight that corresponds to its
//! desired likelihood of being selected next.

use std::collections::HashMap;

use anyhow::{anyhow, Result};
use chrono::{NaiveDateTime, TimeDelta};
use diesel::{
    dsl::exists,
    prelude::*,
    sql_types::{self, Bool},
    QueryDsl,
};
use rand::RngExt;

use super::Library;
use crate::db::{self, models::*, schema::*, tables, views::*};

/// How strong the preference setting affects the selection.
const PREFERENCE_STRENGTH: f64 = 10.0;

/// Parameters for [`Library::generate_recording`], describing a playback "program".
///
/// The filters restrict which recordings may be chosen; the rest shape the odds
/// among those that qualify. A default value is neutral for every field: no
/// filtering, no preference, no repetition penalty.
#[derive(Clone, Default, Debug)]
pub struct GenerateRecordingParams {
    pub composer_id: Option<String>,
    pub performer_id: Option<String>,
    pub ensemble_id: Option<String>,
    pub instrument_id: Option<String>,
    pub work_id: Option<String>,
    pub tag_id: Option<String>,
    pub tag_value: Option<String>,
    /// How much to prefer recordings added to the library recently, from 0.0 to 1.0.
    pub prefer_recently_added: f64,
    /// How much to prefer recordings that have not been played in a long time,
    /// from 0.0 to 1.0.
    pub prefer_least_recently_played: f64,
    /// For how many **minutes** after hearing a composer to avoid them. 0 disables it.
    pub avoid_repeated_composers: i32,
    /// For how many **minutes** after hearing an instrument to avoid it. 0 disables it.
    pub avoid_repeated_instruments: i32,
}

/// One recording a program allows, with everything needed to weight it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Candidate {
    recording_id: String,
    created_at: NaiveDateTime,
    last_played_at: Option<NaiveDateTime>,
}

/// When a candidate's composers and instruments were last heard.
///
/// Only entries within the program's avoidance window are collected, because
/// anything older carries no penalty and would only make the maps bigger.
/// A recording missing from a map has nothing to answer for.
#[derive(Default, Debug)]
struct Repetition {
    composers: HashMap<String, NaiveDateTime>,
    instruments: HashMap<String, NaiveDateTime>,
}

/// Rank `values` from 0.0 for the smallest to 1.0 for the largest.
///
/// Equal values share the average of the ranks they span, so that a set of
/// identical values scores 0.5 throughout rather than being spread out by
/// whatever order they happened to arrive in.
fn rank_ascending<T: Ord + Copy>(values: &[T]) -> Vec<f64> {
    let mut order = (0..values.len()).collect::<Vec<_>>();
    order.sort_by_key(|&i| values[i]);

    let mut ranks = vec![0.0; values.len()];

    if values.len() < 2 {
        // A single candidate is neither first nor last; anything else would
        // make a one-recording program depend on a preference setting.
        ranks.fill(1.0);
        return ranks;
    }

    let last = (values.len() - 1) as f64;
    let mut start = 0;

    while start < order.len() {
        let mut end = start + 1;
        while end < order.len() && values[order[end]] == values[order[start]] {
            end += 1;
        }

        let shared = ((start + end - 1) as f64 / 2.0) / last;
        for &i in &order[start..end] {
            ranks[i] = shared;
        }

        start = end;
    }

    ranks
}

/// Rank `values` from 1.0 for the smallest to 0.0 for the largest.
fn rank_descending<T: Ord + Copy>(values: &[T]) -> Vec<f64> {
    rank_ascending(values)
        .into_iter()
        .map(|rank| 1.0 - rank)
        .collect()
}

/// How much `score` should be weighted based on `preference`.
///
/// `preference` is `[0; 1]` with 1 being the strongest preference.
/// `score` is `[0; 1]` with 1 being the most desirable value.
fn weight_preference(preference: f64, score: f64) -> f64 {
    (PREFERENCE_STRENGTH * preference * (score - 1.0)).exp()
}

/// Weight between `[0; 1]` taking into account `last_played_at`.
///
/// This interpolated linearily between 1.0 for items played >= `window_minutes`
/// ago and 0.0 for items played just now. Items with `last_played_at = None`
/// will get 1.0.
fn weight_avoidance(
    last_played_at: Option<NaiveDateTime>,
    now: NaiveDateTime,
    window_minutes: i32,
) -> f64 {
    if window_minutes <= 0 {
        return 1.0;
    }

    match last_played_at {
        None => 1.0,
        Some(last_played_at) => {
            let elapsed = (now - last_played_at).num_seconds() as f64 / 60.0;
            (elapsed / window_minutes as f64).clamp(0.0, 1.0)
        }
    }
}

/// Compute the desired likelihood of a recording being played next.
///
/// The resulting value will be in the range [0; 1].
fn weight(
    candidate: &Candidate,
    params: &GenerateRecordingParams,
    least_recently_played_score: f64,
    recently_created_score: f64,
    repetition: &Repetition,
    now: NaiveDateTime,
) -> f64 {
    weight_preference(
        params.prefer_least_recently_played,
        least_recently_played_score,
    ) * weight_preference(params.prefer_recently_added, recently_created_score)
        * weight_avoidance(
            repetition.composers.get(&candidate.recording_id).copied(),
            now,
            params.avoid_repeated_composers,
        )
        * weight_avoidance(
            repetition.instruments.get(&candidate.recording_id).copied(),
            now,
            params.avoid_repeated_instruments,
        )
}

/// Draw one candidate, with a probability proportional to its weight.
///
/// Returns `None` for empty `candidates`. Falls back to uniform selection in
/// edge cases.
fn choose<'a>(candidates: &'a [Candidate], weights: &[f64]) -> Option<&'a Candidate> {
    if candidates.is_empty() {
        return None;
    }

    let mut rng = rand::rng();
    let total = weights.iter().sum::<f64>();

    if !total.is_finite() || total <= 0.0 {
        return candidates.get(rng.random_range(0..candidates.len()));
    }

    let mut remaining = rng.random_range(0.0..total);

    for (candidate, weight) in candidates.iter().zip(weights) {
        remaining -= weight;

        if remaining < 0.0 {
            return Some(candidate);
        }
    }

    candidates.last()
}

impl Library {
    /// Choose a recording to play, following `params`.
    pub fn generate_recording(&self, params: &GenerateRecordingParams) -> Result<Recording> {
        let now = db::now();

        let candidates = self.candidates(params)?;
        let repetition = self.repetition(params, now)?;

        let least_recently_played_ranks = rank_descending(
            &candidates
                .iter()
                .map(|c| c.last_played_at.unwrap_or(NaiveDateTime::MIN))
                .collect::<Vec<_>>(),
        );

        let recently_created_ranks =
            rank_ascending(&candidates.iter().map(|c| c.created_at).collect::<Vec<_>>());

        let weights = candidates
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                weight(
                    candidate,
                    params,
                    least_recently_played_ranks[index],
                    recently_created_ranks[index],
                    &repetition,
                    now,
                )
            })
            .collect::<Vec<_>>();

        let chosen = choose(&candidates, &weights)
            .ok_or_else(|| anyhow!("No recording in the library matches this program"))?;

        let connection = &mut *self.conn();

        let row = recordings::table
            .filter(recordings::recording_id.eq(&chosen.recording_id))
            .select(tables::Recording::as_select())
            .first::<tables::Recording>(connection)?;

        Recording::from_table(row, connection)
    }

    /// Every recording the program allows.
    fn candidates(&self, params: &GenerateRecordingParams) -> Result<Vec<Candidate>> {
        let connection = &mut *self.conn();

        let mut query = recordings::table
            .left_join(recording_last_played::table)
            .into_boxed();

        if let Some(composer_id) = &params.composer_id {
            query = query.filter(exists(
                work_persons::table.filter(
                    work_persons::work_id
                        .eq(recordings::work_id)
                        .and(work_persons::person_id.eq(composer_id)),
                ),
            ));
        }

        if let Some(performer_id) = &params.performer_id {
            query =
                query.filter(
                    exists(
                        recording_persons::table.filter(
                            recording_persons::recording_id
                                .eq(recordings::recording_id)
                                .and(recording_persons::person_id.eq(performer_id)),
                        ),
                    )
                    .or(exists(
                        recording_ensembles::table
                            .inner_join(ensemble_persons::table.on(
                                ensemble_persons::ensemble_id.eq(recording_ensembles::ensemble_id),
                            ))
                            .filter(
                                recording_ensembles::recording_id
                                    .eq(recordings::recording_id)
                                    .and(ensemble_persons::person_id.eq(performer_id)),
                            ),
                    )),
                );
        }

        if let Some(ensemble_id) = &params.ensemble_id {
            query = query.filter(exists(
                recording_ensembles::table.filter(
                    recording_ensembles::recording_id
                        .eq(recordings::recording_id)
                        .and(recording_ensembles::ensemble_id.eq(ensemble_id)),
                ),
            ));
        }

        if let Some(instrument_id) = &params.instrument_id {
            query =
                query.filter(
                    exists(
                        work_instruments::table.filter(
                            work_instruments::work_id
                                .eq(recordings::work_id)
                                .and(work_instruments::instrument_id.eq(instrument_id)),
                        ),
                    )
                    .or(exists(
                        recording_persons::table.filter(
                            recording_persons::recording_id
                                .eq(recordings::recording_id)
                                .and(recording_persons::instrument_id.eq(instrument_id)),
                        ),
                    ))
                    .or(exists(
                        recording_ensembles::table
                            .inner_join(ensemble_persons::table.on(
                                ensemble_persons::ensemble_id.eq(recording_ensembles::ensemble_id),
                            ))
                            .filter(
                                recording_ensembles::recording_id
                                    .eq(recordings::recording_id)
                                    .and(ensemble_persons::instrument_id.eq(instrument_id)),
                            ),
                    )),
                );
        }

        if let Some(work_id) = &params.work_id {
            // Matches this work or one of its parts (see `recording_covers_work_condition`),
            // or an arrangement derived from it in either direction.
            query = query.filter(
                super::query::recording_covers_work_condition(work_id).or(
                    diesel::dsl::sql::<Bool>(
                        "EXISTS (SELECT 1 FROM works \
                          WHERE works.work_id = recordings.work_id \
                          AND (works.relates_to = ",
                    )
                    .bind::<sql_types::Text, _>(work_id.clone())
                    .sql(" OR works.work_id = (SELECT relates_to FROM works WHERE work_id = ")
                    .bind::<sql_types::Text, _>(work_id.clone())
                    .sql(")))"),
                ),
            );
        }

        // As in `search`, a tag counts if it is on the recording or on its work,
        // and a valued tag only counts for the exact value.
        if let Some(tag_id) = &params.tag_id {
            query = query.filter(
                diesel::dsl::sql::<Bool>(
                    "(EXISTS (SELECT 1 FROM recording_tags \
                      WHERE recording_tags.recording_id = recordings.recording_id \
                      AND recording_tags.tag_id = ",
                )
                .bind::<sql_types::Text, _>(tag_id.clone())
                .sql(" AND (")
                .bind::<sql_types::Nullable<sql_types::Text>, _>(params.tag_value.clone())
                .sql(" IS NULL OR recording_tags.value = ")
                .bind::<sql_types::Nullable<sql_types::Text>, _>(params.tag_value.clone())
                .sql(
                    ")) OR EXISTS (SELECT 1 FROM work_tags \
                     WHERE work_tags.work_id = recordings.work_id \
                     AND work_tags.tag_id = ",
                )
                .bind::<sql_types::Text, _>(tag_id.clone())
                .sql(" AND (")
                .bind::<sql_types::Nullable<sql_types::Text>, _>(params.tag_value.clone())
                .sql(" IS NULL OR work_tags.value = ")
                .bind::<sql_types::Nullable<sql_types::Text>, _>(params.tag_value.clone())
                .sql(")))"),
            );
        }

        // Do not include empty recordings.
        query = query.filter(exists(
            tracks::table.filter(tracks::recording_id.eq(recordings::recording_id)),
        ));

        let rows = query
            .select((
                recordings::recording_id,
                recordings::created_at,
                recording_last_played::last_played_at.nullable(),
            ))
            .load::<(String, NaiveDateTime, Option<NaiveDateTime>)>(connection)?;

        Ok(rows
            .into_iter()
            .map(|(recording_id, created_at, last_played_at)| Candidate {
                recording_id,
                created_at,
                last_played_at,
            })
            .collect())
    }

    /// When each recording's composers and instruments were last heard, for
    /// those heard recently enough to still count against it.
    fn repetition(
        &self,
        params: &GenerateRecordingParams,
        now: NaiveDateTime,
    ) -> Result<Repetition> {
        let connection = &mut *self.conn();
        let mut repetition = Repetition::default();

        if params.avoid_repeated_composers > 0 {
            let cutoff = now - TimeDelta::minutes(params.avoid_repeated_composers as i64);

            let rows = recordings::table
                .inner_join(work_persons::table.on(work_persons::work_id.eq(recordings::work_id)))
                .inner_join(
                    person_last_played::table
                        .on(person_last_played::person_id.eq(work_persons::person_id)),
                )
                .filter(person_last_played::last_played_at.ge(cutoff))
                .select((recordings::recording_id, person_last_played::last_played_at))
                .load::<(String, NaiveDateTime)>(connection)?;

            repetition.composers = most_recent(rows);
        }

        if params.avoid_repeated_instruments > 0 {
            let cutoff = now - TimeDelta::minutes(params.avoid_repeated_instruments as i64);

            let rows = recordings::table
                .inner_join(
                    work_instruments::table.on(work_instruments::work_id.eq(recordings::work_id)),
                )
                .inner_join(
                    instrument_last_played::table
                        .on(instrument_last_played::instrument_id
                            .eq(work_instruments::instrument_id)),
                )
                .filter(instrument_last_played::last_played_at.ge(cutoff))
                .select((
                    recordings::recording_id,
                    instrument_last_played::last_played_at,
                ))
                .load::<(String, NaiveDateTime)>(connection)?;

            repetition.instruments = most_recent(rows);
        }

        Ok(repetition)
    }

    /// Record that a track was played.
    pub fn track_played(&self, track_id: &str) -> Result<()> {
        let connection = &mut *self.conn();

        connection.transaction::<(), diesel::result::Error, _>(|connection| {
            let recording_id = tracks::table
                .filter(tracks::track_id.eq(track_id))
                .select(tracks::recording_id)
                .first::<String>(connection)?;

            diesel::insert_into(plays::table)
                .values(tables::Play {
                    play_id: db::generate_id(),
                    track_id: Some(track_id.to_owned()),
                    recording_id,
                    played_at: db::now(),
                })
                .execute(connection)?;

            Ok(())
        })?;

        Ok(())
    }
}

/// Keep the latest timestamp per key.
fn most_recent(rows: Vec<(String, NaiveDateTime)>) -> HashMap<String, NaiveDateTime> {
    let mut latest = HashMap::new();

    for (key, at) in rows {
        latest
            .entry(key)
            .and_modify(|current: &mut NaiveDateTime| {
                if at > *current {
                    *current = at;
                }
            })
            .or_insert(at);
    }

    latest
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, fs};

    use tempfile::TempDir;

    use super::*;
    use crate::db::TranslatedString;

    fn translated(name: &str) -> TranslatedString {
        let mut translations = HashMap::new();
        translations.insert("generic".to_string(), name.to_string());
        TranslatedString(translations)
    }

    fn library(dir: &TempDir, cache_dir: &TempDir) -> Library {
        Library::new(dir.path(), cache_dir.path()).unwrap()
    }

    /// A work with one recording that has one track, so it is a valid candidate.
    fn work_with_recording(
        library: &Library,
        source_dir: &TempDir,
        name: &str,
        relates_to: Option<Work>,
    ) -> (Work, Recording) {
        let work = library
            .create_work(
                translated(name),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                relates_to,
                true,
            )
            .unwrap();

        let recording = library
            .create_recording(work.clone(), Vec::new(), Vec::new(), Vec::new(), None, true)
            .unwrap();

        let source = source_dir.path().join(format!("{name}.mp3"));
        fs::write(&source, format!("audio of {name}").as_bytes()).unwrap();

        library
            .import_track(&source, &recording.recording_id, 0, Vec::new())
            .unwrap();

        (work, recording)
    }

    fn recording_ids(candidates: Vec<Candidate>) -> HashSet<String> {
        candidates.into_iter().map(|c| c.recording_id).collect()
    }

    #[test]
    fn a_work_query_also_matches_what_was_derived_from_it() {
        let dir = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        let source_dir = TempDir::new().unwrap();
        let library = library(&dir, &cache_dir);

        let (original, original_recording) =
            work_with_recording(&library, &source_dir, "Original", None);
        let (_, arrangement_recording) =
            work_with_recording(&library, &source_dir, "Arrangement", Some(original.clone()));
        let (_, unrelated_recording) =
            work_with_recording(&library, &source_dir, "Unrelated", None);

        let params = GenerateRecordingParams {
            work_id: Some(original.work_id.clone()),
            ..Default::default()
        };

        let recording_ids = recording_ids(library.candidates(&params).unwrap());

        assert!(recording_ids.contains(&original_recording.recording_id));
        assert!(recording_ids.contains(&arrangement_recording.recording_id));
        assert!(!recording_ids.contains(&unrelated_recording.recording_id));
    }

    #[test]
    fn a_work_query_also_reaches_back_to_what_it_was_derived_from() {
        let dir = TempDir::new().unwrap();
        let cache_dir = TempDir::new().unwrap();
        let source_dir = TempDir::new().unwrap();
        let library = library(&dir, &cache_dir);

        let (original, original_recording) =
            work_with_recording(&library, &source_dir, "Original", None);
        let (arrangement, arrangement_recording) =
            work_with_recording(&library, &source_dir, "Arrangement", Some(original.clone()));
        let (_, unrelated_recording) =
            work_with_recording(&library, &source_dir, "Unrelated", None);

        let params = GenerateRecordingParams {
            work_id: Some(arrangement.work_id.clone()),
            ..Default::default()
        };

        let recording_ids = recording_ids(library.candidates(&params).unwrap());

        assert!(recording_ids.contains(&original_recording.recording_id));
        assert!(recording_ids.contains(&arrangement_recording.recording_id));
        assert!(!recording_ids.contains(&unrelated_recording.recording_id));
    }
}
