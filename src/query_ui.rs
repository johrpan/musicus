//! UI presentation for search queries.
//!
//! `LibraryQuery` carries only IDs; the title, description and highlight of a
//! query need the entities' names, so those are resolved here, in the
//! application, from the library. Keeping this formatting in the consumer
//! leaves the library API free of locale-aware display concerns.

use gettextrs::gettext;
use musicus_library::{
    format_translated,
    library::{Facet, LibraryQuery, TagFilter},
    models::TagValue,
    Library,
};

/// How many items a search page shows per facet.
pub const SEARCH_RESULTS_PER_FACET: usize = 9;

/// The item that the query is mainly about, loaded from the library.
///
/// Returns `None` if the query is empty or its main item cannot be loaded.
pub fn query_highlight(library: &Library, query: &LibraryQuery) -> Option<Facet> {
    if let Some(work_id) = &query.work {
        library.work(work_id).ok().map(Facet::Work)
    } else if let Some(person_id) = &query.work_person {
        library.person(person_id).ok().map(Facet::WorkPerson)
    } else if let Some(person_id) = &query.recording_person {
        library.person(person_id).ok().map(Facet::RecordingPerson)
    } else if let Some(ensemble_id) = &query.ensemble {
        library.ensemble(ensemble_id).ok().map(Facet::Ensemble)
    } else if let Some(instrument_id) = &query.instrument {
        library
            .instrument(instrument_id)
            .ok()
            .map(Facet::Instrument)
    } else if let Some(tag) = &query.tag {
        query_tag_value(library, tag).map(|tag_value| Facet::Tag(tag_value))
    } else {
        None
    }
}

/// A short name for what the query selects.
pub fn query_title(library: &Library, query: &LibraryQuery) -> Option<String> {
    Some(match query_highlight(library, query)? {
        Facet::Work(work) => work.name.get().to_owned(),
        Facet::WorkPerson(person) | Facet::RecordingPerson(person) => person.name.get().to_owned(),
        Facet::Ensemble(ensemble) => ensemble.name.get().to_owned(),
        Facet::Instrument(instrument) => {
            format_translated!(gettext("Music for {}"), instrument.name.get())
        }
        Facet::Tag(tag) => tag.to_string(),
    })
}

/// The parts of the query that are not already covered by its title.
pub fn query_description(library: &Library, query: &LibraryQuery) -> Option<String> {
    let mut details = Vec::new();

    match query_highlight(library, query)? {
        Facet::Work(work) => {
            if let Some(work_persons) = work.work_persons_string() {
                details.push(work_persons);
            }
        }
        Facet::WorkPerson(_) => {
            if let Some(instrument_id) = &query.instrument {
                if let Ok(instrument) = library.instrument(instrument_id) {
                    details.push(format_translated!(gettext("Works with {}"), instrument));
                }
            }

            if let (Some(person_id), Some(ensemble_id)) = (&query.recording_person, &query.ensemble)
            {
                if let (Ok(person), Ok(ensemble)) =
                    (library.person(person_id), library.ensemble(ensemble_id))
                {
                    details.push(format_translated!(
                        gettext("Performed by {} and {}"),
                        person,
                        ensemble
                    ));
                }
            } else if let Some(person_id) = &query.recording_person {
                if let Ok(person) = library.person(person_id) {
                    details.push(format_translated!(gettext("Performed by {}"), person));
                }
            } else if let Some(ensemble_id) = &query.ensemble {
                if let Ok(ensemble) = library.ensemble(ensemble_id) {
                    details.push(format_translated!(gettext("Performed by {}"), ensemble));
                }
            }
        }
        Facet::RecordingPerson(_) => {
            if let Some(instrument_id) = &query.instrument {
                if let Ok(instrument) = library.instrument(instrument_id) {
                    details.push(format_translated!(gettext("Works with {}"), instrument));
                }
            }

            if let Some(ensemble_id) = &query.ensemble {
                if let Ok(ensemble) = library.ensemble(ensemble_id) {
                    details.push(format_translated!(gettext("Performed with {}"), ensemble));
                }
            }
        }
        Facet::Ensemble(ensemble) => {
            if let Some(instrument_id) = &query.instrument {
                if let Ok(instrument) = library.instrument(instrument_id) {
                    details.push(format_translated!(gettext("Works with {}"), instrument));
                }
            }

            if let Some(members) = ensemble.members_string() {
                details.push(format_translated!(gettext("Members: {}"), members));
            }
        }
        Facet::Instrument(_) => (),
        Facet::Tag(_) => {
            if let Some(instrument_id) = &query.instrument {
                if let Ok(instrument) = library.instrument(instrument_id) {
                    details.push(format_translated!(gettext("Works with {}"), instrument));
                }
            }

            if let Some(person_id) = &query.recording_person {
                if let Ok(person) = library.person(person_id) {
                    details.push(format_translated!(gettext("Performed by {}"), person));
                }
            } else if let Some(ensemble_id) = &query.ensemble {
                if let Ok(ensemble) = library.ensemble(ensemble_id) {
                    details.push(format_translated!(gettext("Performed by {}"), ensemble));
                }
            }
        }
    }

    if let Some(tag) = &query.tag {
        if !matches!(query_highlight(library, query), Some(Facet::Tag(_))) {
            if let Some(tag_value) = query_tag_value(library, tag) {
                details.push(tag_value.to_string());
            }
        }
    }

    if details.is_empty() {
        None
    } else {
        Some(details.join(", "))
    }
}

/// The tag filter as a `TagValue` model, for display.
fn query_tag_value(library: &Library, tag: &TagFilter) -> Option<TagValue> {
    let tag_model = library.tag(&tag.tag_id).ok()?;

    Some(TagValue {
        tag: tag_model,
        value: tag.value.clone(),
    })
}

/// The filter a `TagValue` facet becomes.
pub fn tag_filter_of(tag: &TagValue) -> TagFilter {
    TagFilter {
        tag_id: tag.tag.id.clone(),
        value: tag.value.clone(),
    }
}
