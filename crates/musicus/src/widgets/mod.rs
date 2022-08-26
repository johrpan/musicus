use gtk::prelude::*;

pub mod button_row;
pub use button_row::*;

pub mod editor;
pub use editor::*;

pub mod list;
pub use list::*;

pub mod player_bar;
pub use player_bar::*;

pub mod screen;
pub use screen::*;

pub mod section;
pub use section::*;

pub mod track_row;
pub use track_row::TrackRow;

mod indexed_list_model;

/// Something that can be represented as a GTK widget.
pub trait Widget {
    /// Get the widget.
    fn get_widget(&self) -> gtk::Widget;
}

impl<W: IsA<gtk::Widget>> Widget for W {
    fn get_widget(&self) -> gtk::Widget {
        self.clone().upcast()
    }
}
