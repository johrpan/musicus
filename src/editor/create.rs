//! Guided creation of entities that depend on other entities.
//!
//! A selector lets the user create the intermediate entities of a multistep selection
//! first. Creating a recording may therefore mean showing a person editor, a work editor
//! and a recording editor in sequence, each pre-filled with the results of the previous
//! one.

use std::rc::Rc;

use musicus_library::db::models::{Person, Recording, Work};

use crate::{
    editor::{recording::RecordingEditor, simple_entity::SimpleEntityEditor, work::WorkEditor},
    library::Library,
    selector::{ComposerPrefill, RecordingPrefill, RecordingWork, WorkPrefill},
};

/// Let the user create a new work, preceded by an editor for its composer if that is what
/// they chose within the selector.
pub fn work<F: Fn(Work) + 'static>(
    navigation: &adw::NavigationView,
    library: &Library,
    prefill: WorkPrefill,
    handler: F,
) {
    push_work(navigation, library, prefill, Rc::new(handler));
}

/// Let the user create a new recording, preceded by editors for its work and that work's
/// composer if that is what they chose within the selector.
pub fn recording<F: Fn(Recording) + 'static>(
    navigation: &adw::NavigationView,
    library: &Library,
    prefill: RecordingPrefill,
    handler: F,
) {
    push_recording(navigation, library, prefill, Rc::new(handler));
}

/// The handler is boxed, because the steps are pushed recursively.
type Handler<T> = Rc<dyn Fn(T)>;

fn push_work(
    navigation: &adw::NavigationView,
    library: &Library,
    prefill: WorkPrefill,
    handler: Handler<Work>,
) {
    if let ComposerPrefill::New(composer_name) = &prefill.composer {
        let editor = SimpleEntityEditor::person(navigation, library, None);
        editor.set_name(composer_name);

        let navigation_ = navigation.to_owned();
        let library_ = library.to_owned();
        let work_name = prefill.name.to_owned();

        editor.connect_created(move |_, composer: Person| {
            let prefill = WorkPrefill {
                composer: ComposerPrefill::Person(composer),
                name: work_name.to_owned(),
            };

            push_work(&navigation_, &library_, prefill, Rc::clone(&handler));
        });

        navigation.push(&editor);
        return;
    }

    let editor = WorkEditor::new(navigation, library, None, false);
    editor.prefill(&prefill);
    editor.connect_created(move |_, work| handler(work));

    navigation.push(&editor);
}

fn push_recording(
    navigation: &adw::NavigationView,
    library: &Library,
    prefill: RecordingPrefill,
    handler: Handler<Recording>,
) {
    if let RecordingWork::New(work_prefill) = prefill.work {
        let navigation_ = navigation.to_owned();
        let library_ = library.to_owned();

        push_work(
            navigation,
            library,
            work_prefill,
            Rc::new(move |work: Work| {
                let prefill = RecordingPrefill {
                    work: RecordingWork::Work(work),
                };

                push_recording(&navigation_, &library_, prefill, Rc::clone(&handler));
            }),
        );

        return;
    }

    let editor = RecordingEditor::new(navigation, library, None);
    editor.prefill(&prefill);
    editor.connect_created(move |_, recording| handler(recording));

    navigation.push(&editor);
}
