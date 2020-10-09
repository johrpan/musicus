use super::*;
use crate::backend::Backend;
use crate::database::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::get_widget;
use std::cell::RefCell;
use std::rc::Rc;
use std::convert::TryInto;

pub struct RecordingEditor<F>
where
    F: Fn(RecordingDescription) -> () + 'static,
{
    backend: Rc<Backend>,
    window: gtk::Window,
    callback: F,
    id: i64,
    save_button: gtk::Button,
    work_label: gtk::Label,
    work: RefCell<Option<WorkDescription>>,
    comment_entry: gtk::Entry,
    performers: RefCell<Vec<PerformanceDescription>>,
    performer_list: gtk::ListBox,
}

impl<F> RecordingEditor<F>
where
    F: Fn(RecordingDescription) -> () + 'static,
{
    pub fn new<P: IsA<gtk::Window>>(
        backend: Rc<Backend>,
        parent: &P,
        recording: Option<RecordingDescription>,
        callback: F,
    ) -> Rc<Self> {
        let builder =
            gtk::Builder::from_resource("/de/johrpan/musicus_editor/ui/recording_editor.ui");

        get_widget!(builder, gtk::Window, window);
        get_widget!(builder, gtk::Button, cancel_button);
        get_widget!(builder, gtk::Button, save_button);
        get_widget!(builder, gtk::Button, work_button);
        get_widget!(builder, gtk::Label, work_label);
        get_widget!(builder, gtk::Entry, comment_entry);
        get_widget!(builder, gtk::ListBox, performer_list);
        get_widget!(builder, gtk::Button, add_performer_button);
        get_widget!(builder, gtk::Button, remove_performer_button);

        let (id, work, performers) = match recording {
            Some(recording) => {
                save_button.set_sensitive(true);
                (recording.id, Some(recording.work), recording.performances)
            }
            None => (rand::random::<u32>().into(), None, Vec::new()),
        };

        let result = Rc::new(RecordingEditor {
            backend: backend,
            window: window,
            callback: callback,
            id: id,
            save_button: save_button,
            work_label: work_label,
            work: RefCell::new(work),
            comment_entry: comment_entry,
            performers: RefCell::new(performers),
            performer_list: performer_list,
        });

        cancel_button.connect_clicked(clone!(@strong result => move |_| {
            result.window.close();
        }));

        result
            .save_button
            .connect_clicked(clone!(@strong result => move |_| {
                let recording = RecordingDescription {
                    id: result.id,
                    work: result.work.borrow().clone().expect("Tried to create recording without work!"),
                    comment: result.comment_entry.get_text().to_string(),
                    performances: result.performers.borrow().to_vec(),
                };
    
                result.backend.update_recording(recording.clone().into(), clone!(@strong result => move |_| {
                    result.window.close();
                    (result.callback)(recording.clone());
                }));
    
                result.window.close();
            }));

        work_button.connect_clicked(clone!(@strong result => move |_| {
            WorkSelector::new(result.backend.clone(), &result.window, clone!(@strong result => move |work| {
                result.work.replace(Some(work.clone()));
                result.work_label.set_text(&format!("{}: {}", work.composer.name_fl(), work.title));
                result.save_button.set_sensitive(true);
            })).show();
        }));

        add_performer_button.connect_clicked(clone!(@strong result => move |_| {
            PerformanceEditor::new(result.backend.clone(), &result.window, None, clone!(@strong result => move |performance| {
                {
                    let mut performers = result.performers.borrow_mut();
                    performers.push(performance);
                }
                
                result.show_performers();
            })).show();
        }));

        remove_performer_button.connect_clicked(clone!(@strong result => move |_| {
            let row = result.get_selected_performer_row();
            match row {
                Some(row) => {
                    let index = row.get_index();
                    let index: usize = index.try_into().unwrap();
                    result.performers.borrow_mut().remove(index);
                    result.show_performers();
                }
                None => (),
            }
        }));

        result.window.set_transient_for(Some(parent));

        result
    }

    pub fn show(&self) {
        self.window.show();
    }

    fn show_performers(&self) {
        for child in self.performer_list.get_children() {
            self.performer_list.remove(&child);
        }

        for (index, performer) in self.performers.borrow().iter().enumerate() {
            let label = gtk::Label::new(Some(&performer.get_title()));
            label.set_halign(gtk::Align::Start);
            let row = SelectorRow::new(index.try_into().unwrap(), &label);
            row.show_all();
            self.performer_list.insert(&row, -1);
        }
    }

    fn get_selected_performer_row(&self) -> Option<SelectorRow> {
        match self.performer_list.get_selected_rows().first() {
            Some(row) => match row.get_child() {
                Some(child) => Some(child.downcast().unwrap()),
                None => None,
            },
            None => None,
        }
    }
}
