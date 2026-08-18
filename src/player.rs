use std::{
    cell::{Cell, OnceCell, RefCell},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use fragile::Fragile;
use gettextrs::gettext;
use gstreamer_play::gst;
use gtk::{
    gio,
    glib::{self, clone, subclass::Signal, Properties},
    prelude::*,
    subclass::prelude::*,
};
use musicus_library::{
    db::models::{Recording, Track, Work},
    format_translated,
};
use once_cell::sync::Lazy;

use crate::{
    config,
    library::{GenerateRecordingParams, Library},
    playlist_item::PlaylistItem,
    program::Program,
};

/// How many tracks may fail in a row before playback gives up.
const MAX_CONSECUTIVE_ERRORS: u32 = 5;

/// How far into a track it counts as listened to: half of it, or this, whichever
/// comes first.
const PLAY_THRESHOLD_MS: u64 = 4 * 60 * 1000;

mod imp {
    use super::*;

    #[derive(Properties, Debug, Default)]
    #[properties(wrapper_type = super::Player)]
    pub struct Player {
        #[property(get, set)]
        pub library: RefCell<Option<Library>>,
        #[property(get, set)]
        pub active: Cell<bool>,
        #[property(get, set)]
        pub playing: Cell<bool>,
        #[property(get, set = Self::set_program)]
        pub program: RefCell<Option<Program>>,
        #[property(get, construct_only)]
        pub playlist: OnceCell<gio::ListStore>,
        #[property(get, set = Self::set_current_index)]
        pub current_index: Cell<u32>,
        #[property(get, set)]
        pub duration_ms: Cell<u64>,
        #[property(get, set)]
        pub position_ms: Cell<u64>,

        pub play: OnceCell<gstreamer_play::Play>,
        pub play_signal_adapter: OnceCell<gstreamer_play::PlaySignalAdapter>,
        pub mpris_player: OnceCell<mpris_server::Player>,

        /// How many tracks failed in a row without one in between that played.
        pub consecutive_errors: Cell<u32>,
        /// Whether the current item has already been counted as played.
        pub play_reported: Cell<bool>,
    }

    impl Player {
        /// Set the program to play from.
        ///
        /// The player will always use its own copy of the program. Otherwise, changing the
        /// settings of the program that is currently playing would also change the program it
        /// was started from, such as one of the default programs.
        pub fn set_program(&self, program: Option<Program>) {
            self.program.replace(program.map(|p| p.duplicate()));
        }

        pub fn set_current_index(&self, index: u32) {
            let playlist = self.playlist.get().unwrap();

            if let Some(item) = playlist.item(index) {
                if let Some(old_item) = playlist.item(self.current_index.get()) {
                    old_item
                        .downcast::<PlaylistItem>()
                        .unwrap()
                        .set_is_playing(false);
                }

                let item = item.downcast::<PlaylistItem>().unwrap();

                let obj = self.obj().clone();
                let item_clone = item.clone();
                glib::spawn_future_local(async move {
                    let Some(mpris_player) = obj.imp().mpris_player.get() else {
                        return;
                    };

                    if let Err(err) = mpris_player
                        .set_metadata(
                            mpris_server::Metadata::builder()
                                .title(item_clone.make_title())
                                .artist(vec![item_clone
                                    .make_subtitle()
                                    .unwrap_or_else(String::new)])
                                .build(),
                        )
                        .await
                    {
                        log::warn!("Failed to publish track metadata over MPRIS: {err}");
                    }
                });

                let uri = match glib::filename_to_uri(item.path(), None) {
                    Ok(uri) => uri,
                    Err(err) => {
                        log::error!("Failed to build a URI for {}: {err}", item.path().display());
                        self.obj()
                            .report_error(&gettext("This track cannot be played."));
                        return;
                    }
                };

                let play = self.play.get().unwrap();
                play.set_uri(Some(&uri));

                // Everything that describes the current item has to be in place
                // before playback starts, because `playback_started` reads it.
                self.current_index.set(index);
                item.set_is_playing(true);

                self.play_reported.set(false);

                if self.playing.get() {
                    play.play();
                }
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Player {
        const NAME: &'static str = "MusicusPlayer";
        type Type = super::Player;
    }

    #[glib::derived_properties]
    impl ObjectImpl for Player {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("raise").build(),
                    Signal::builder("error")
                        .param_types([String::static_type()])
                        .build(),
                ]
            });

            SIGNALS.as_ref()
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj().clone();
            glib::spawn_future_local(async move {
                match obj.init_mpris().await {
                    Ok(task) => {
                        // Run the MPRIS server
                        task.await;
                    }
                    Err(err) => {
                        log::error!("Failed to initialize MPRIS server: {err:?}");
                    }
                }
            });

            let play = gstreamer_play::Play::new(None::<gstreamer_play::PlayVideoRenderer>);

            let mut config = play.config();
            config.set_position_update_interval(250);
            if let Err(err) = play.set_config(config) {
                log::warn!("Failed to configure the player: {err}");
            }
            play.set_video_track_enabled(false);

            let play_signal_adapter = gstreamer_play::PlaySignalAdapter::new(&play);

            let obj = Fragile::new(self.obj().to_owned());
            play_signal_adapter.connect_end_of_stream(move |_| {
                obj.get().next();
            });

            let obj = Fragile::new(self.obj().to_owned());
            play_signal_adapter.connect_error(move |_, error, _| {
                let obj = obj.get();
                log::error!("Playback failed: {error}");

                let message = match obj.current_item().and_then(|item| item.make_subtitle()) {
                    Some(title) => format_translated!(gettext("Could not play {}."), title),
                    None => gettext("Could not play this track."),
                };

                obj.report_error(&message);
                obj.skip_failed_item();
            });

            let obj = Fragile::new(self.obj().to_owned());
            play_signal_adapter.connect_state_changed(move |_, state| {
                if state == gstreamer_play::PlayState::Playing {
                    obj.get().playback_started();
                }
            });

            let obj = Fragile::new(self.obj().to_owned());
            play_signal_adapter.connect_warning(move |_, error, _| {
                let _ = obj.get();
                log::warn!("Playback warning: {error}");
            });

            let obj = Fragile::new(self.obj().to_owned());
            play_signal_adapter.connect_position_updated(move |_, position| {
                if let Some(position) = position {
                    let obj = obj.get();
                    obj.imp().position_ms.set(position.mseconds());
                    obj.notify_position_ms();
                    obj.report_play_if_listened();
                }
            });

            let obj = Fragile::new(self.obj().to_owned());
            play_signal_adapter.connect_duration_changed(move |_, duration| {
                if let Some(duration) = duration {
                    let obj = obj.get();
                    let imp = obj.imp();

                    imp.position_ms.set(0);
                    obj.notify_position_ms();

                    imp.duration_ms.set(duration.mseconds());
                    obj.notify_duration_ms();
                }
            });

            self.play.set(play).unwrap();
            self.play_signal_adapter.set(play_signal_adapter).unwrap();
        }
    }
}

glib::wrapper! {
    pub struct Player(ObjectSubclass<imp::Player>);
}

impl Player {
    pub fn new() -> Self {
        glib::Object::builder()
            .property("active", false)
            .property("playing", false)
            .property("playlist", gio::ListStore::new::<PlaylistItem>())
            .property("current-index", 0u32)
            .property("position-ms", 0u64)
            .property("duration-ms", 60_000u64)
            .build()
    }

    pub fn connect_raise<F: Fn(&Self) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("raise", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            f(&obj);
            None
        })
    }

    pub fn recording_to_playlist(&self, recording: &Recording) -> Vec<PlaylistItem> {
        let tracks = self.matching_tracks(recording, None);

        if tracks.is_empty() {
            log::warn!("Recording without tracks: {}.", &recording.recording_id);
            return Vec::new();
        }

        let tracks = tracks
            .iter()
            .enumerate()
            .map(|(index, track)| (track, index + 1))
            .collect::<Vec<(&Track, usize)>>();

        self.tracks_to_playlist(recording, &tracks)
    }

    /// Create playlist items for the tracks of `recording` that explicitly carry `work`
    /// (or one of `work`'s own parts) among their assigned works — i.e. only the
    /// track(s) that are actually the part being viewed, not the whole recording.
    /// A track with no assigned works at all stands for the recording's whole work, so
    /// it only matches here when `work` is exactly that whole work.
    pub fn recording_to_playlist_for_work(
        &self,
        recording: &Recording,
        work: &Work,
    ) -> Vec<PlaylistItem> {
        let tracks = self.matching_tracks(recording, Some(work));

        if tracks.is_empty() {
            log::warn!(
                "No tracks of recording {} carry work {}.",
                &recording.recording_id,
                &work.work_id
            );
            return Vec::new();
        }

        let tracks = tracks
            .iter()
            .enumerate()
            .map(|(index, track)| (track, index + 1))
            .collect::<Vec<(&Track, usize)>>();

        self.tracks_to_playlist(recording, &tracks)
    }

    /// Create playlist items for one randomly selected track of `recording`.
    pub fn recording_to_playlist_single_track(&self, recording: &Recording) -> Vec<PlaylistItem> {
        self.playlist_for_single_track(recording, None)
    }

    /// Like [`Self::recording_to_playlist_single_track`], but the track is drawn only
    /// from among those matching `work`, the same way [`Self::recording_to_playlist_for_work`]
    /// restricts them.
    pub fn recording_to_playlist_for_work_single_track(
        &self,
        recording: &Recording,
        work: &Work,
    ) -> Vec<PlaylistItem> {
        self.playlist_for_single_track(recording, Some(work))
    }

    fn playlist_for_single_track(
        &self,
        recording: &Recording,
        work: Option<&Work>,
    ) -> Vec<PlaylistItem> {
        let tracks = self.matching_tracks(recording, work);

        if tracks.is_empty() {
            log::warn!("Recording without tracks: {}.", &recording.recording_id);
            return Vec::new();
        }

        let index = glib::random_int_range(0, tracks.len() as i32) as usize;

        self.tracks_to_playlist(recording, &[(&tracks[index], index + 1)])
    }

    /// The tracks of `recording`, restricted to those explicitly carrying `work` (or
    /// one of its own parts) when `work` is given — see [`Self::recording_to_playlist_for_work`].
    fn matching_tracks(&self, recording: &Recording, work: Option<&Work>) -> Vec<Track> {
        let tracks = self
            .library()
            .unwrap()
            .tracks_for_recording(&recording.recording_id)
            .unwrap();

        match work {
            None => tracks,
            Some(work) => tracks
                .into_iter()
                .filter(|track| {
                    if track.works.is_empty() {
                        work.work_id == recording.work.work_id
                    } else {
                        track.works.iter().any(|w| work.contains(&w.work_id))
                    }
                })
                .collect(),
        }
    }

    /// Create playlist items for the provided tracks of `recording`. Each track is
    /// accompanied by its one based number within the recording. The first item will
    /// be marked as the title item.
    ///
    /// A track explicitly assigned its own part of the work (see `track.works`) is
    /// titled with that part, even if it is the only track. A track with no assigned part
    /// is titled generically ("Track N") only when there is more than one track to tell
    /// apart.
    fn tracks_to_playlist(
        &self,
        recording: &Recording,
        tracks: &[(&Track, usize)],
    ) -> Vec<PlaylistItem> {
        let performances = recording.performers_string();

        let track_title = |track: &Track, number: usize| -> Option<String> {
            let title = track
                .works
                .iter()
                .map(|w| w.name.get().to_string())
                .collect::<Vec<String>>()
                .join(", ");

            if !title.is_empty() {
                Some(title)
            } else if tracks.len() > 1 {
                Some(format!("Track {number}"))
            } else {
                None
            }
        };

        tracks
            .iter()
            .enumerate()
            .map(|(index, (track, number))| {
                PlaylistItem::new(
                    index == 0,
                    recording.work.composers_string(),
                    recording.work.name.get(),
                    Some(&performances),
                    track_title(track, *number).as_deref(),
                    self.library_path_to_file_path(&track.path),
                    &track.track_id,
                )
            })
            .collect()
    }

    /// Append playlist items to the playlist and return the index of the first newly added item.
    /// An error will be returned if `items` is empty.
    pub fn append(&self, items: Vec<PlaylistItem>) -> Result<u32> {
        if !items.is_empty() {
            let playlist = self.playlist();
            let first_index = playlist.n_items();

            for item in items {
                playlist.append(&item);
            }

            // If playlist was empty:
            if first_index == 0 {
                self.set_active(true);
                self.set_current_index(0);
                self.pause();
            }

            Ok(first_index)
        } else {
            Err(anyhow!("At least one item has to be added to the playlist"))
        }
    }

    /// Append playlist items to the playlist and immediately start playing the first newly added
    /// item. This will discard the error if `items` is empty.
    pub fn append_and_play(&self, items: Vec<PlaylistItem>) {
        match self.append(items) {
            Ok(index) => {
                self.set_current_index(index);
                self.play();
            }
            Err(err) => {
                log::warn!("Failed to append and play items: {err:?}");
            }
        }
    }

    /// Generate new playlist items based on the current program and immediately start playing the
    /// first new item.
    pub fn play_from_program(&self) {
        if let Some(program) = self.program() {
            match self.generate_items(&program) {
                Ok(index) => {
                    self.set_current_index(index);
                    self.play();
                }
                Err(err) => {
                    log::warn!("Failed to play from program: {err:?}");
                    self.report_error(&gettext("No music matches this program."));
                }
            }
        }
    }

    /// Stop generating new playlist items from the current program. Neither the current playback
    /// nor the items that are already part of the playlist will be affected.
    pub fn cancel_program(&self) {
        self.imp().program.replace(None);
        self.notify_program();
    }

    pub fn play_pause(&self) {
        if self.playing() {
            self.pause();
        } else {
            self.play();
        }
    }

    pub fn play(&self) {
        let imp = self.imp();
        imp.play.get().unwrap().play();
        self.set_playing(true);
        self.publish_playback_status(mpris_server::PlaybackStatus::Playing);
    }

    pub fn pause(&self) {
        let imp = self.imp();
        imp.play.get().unwrap().pause();
        self.set_playing(false);
        self.publish_playback_status(mpris_server::PlaybackStatus::Paused);
    }

    fn publish_playback_status(&self, status: mpris_server::PlaybackStatus) {
        let obj = self.clone();
        glib::spawn_future_local(async move {
            let Some(mpris_player) = obj.imp().mpris_player.get() else {
                return;
            };

            if let Err(err) = mpris_player.set_playback_status(status).await {
                log::warn!("Failed to publish playback status over MPRIS: {err}");
            }
        });
    }

    fn playback_started(&self) {
        self.imp().consecutive_errors.set(0);
    }

    /// Record the current item as played, once enough of it has gone by.
    ///
    /// Called from the position updates, which arrive every 250 ms.
    /// `play_reported` is reset per item in `set_current_index`, so an item is
    /// recorded at most once no matter how often this runs.
    ///
    /// Seeking past the threshold counts as having listened. Being exact would
    /// mean accumulating playback time separately; going by position is what
    /// most players do and is close enough for statistics.
    fn report_play_if_listened(&self) {
        let imp = self.imp();

        if imp.play_reported.get() {
            return;
        }

        let duration_ms = imp.duration_ms.get();
        if duration_ms == 0 {
            return;
        }

        if imp.position_ms.get() < (duration_ms / 2).min(PLAY_THRESHOLD_MS) {
            return;
        }

        imp.play_reported.set(true);

        let Some(item) = self.current_item() else {
            return;
        };

        if let Some(library) = imp.library.borrow().as_ref() {
            if let Err(err) = library.track_played(&item.track_id()) {
                log::warn!("Failed to record that a track was played: {err:?}");
            }
        }
    }

    /// Continue after the current item failed to play.
    fn skip_failed_item(&self) {
        let imp = self.imp();

        let consecutive_errors = imp.consecutive_errors.get() + 1;
        imp.consecutive_errors.set(consecutive_errors);

        if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
            log::error!("Stopping playback after {consecutive_errors} tracks failed in a row");
            imp.consecutive_errors.set(0);
            self.pause();
            return;
        }

        // A program keeps generating items, so there is always something to
        // skip to.
        if self.current_index() + 1 < self.playlist().n_items() || self.program().is_some() {
            self.next();
        } else {
            self.pause();
        }
    }

    pub fn report_error(&self, message: &str) {
        self.emit_by_name::<()>("error", &[&message.to_owned()]);
    }

    pub fn connect_error<F: Fn(&Self, String) + 'static>(&self, f: F) -> glib::SignalHandlerId {
        self.connect_local("error", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let message = values[1].get::<String>().unwrap();
            f(&obj, message);
            None
        })
    }

    pub fn seek_to(&self, time_ms: u64) {
        let imp = self.imp();
        imp.play
            .get()
            .unwrap()
            .seek(gst::ClockTime::from_mseconds(time_ms));
    }

    pub fn current_item(&self) -> Option<PlaylistItem> {
        let imp = self.imp();
        imp.playlist
            .get()
            .unwrap()
            .item(imp.current_index.get())
            .and_downcast::<PlaylistItem>()
    }

    pub fn next(&self) {
        if self.current_index() + 1 < self.playlist().n_items() {
            self.set_current_index(self.current_index() + 1);
        } else if let Some(program) = self.program() {
            match self.generate_items(&program) {
                Ok(index) => self.set_current_index(index),
                Err(err) => {
                    log::warn!("Failed to continue playing from program: {err:?}");
                    self.report_error(&gettext("No more music matches this program."));
                    self.pause();
                }
            }
        }
    }

    pub fn previous(&self) {
        if self.current_index() > 0 {
            self.set_current_index(self.current_index() - 1);
        }
    }

    async fn init_mpris(&self) -> Result<mpris_server::LocalServerRunTask> {
        let mpris_player = mpris_server::Player::builder(config::APP_ID)
            .desktop_entry(config::APP_ID)
            .can_raise(true)
            .can_play(true)
            .can_pause(true)
            .can_go_previous(true)
            .can_go_next(true)
            .build()
            .await?;

        mpris_player.connect_raise(clone!(
            #[weak(rename_to = obj)]
            self,
            move |_| obj.emit_by_name::<()>("raise", &[])
        ));

        mpris_player.connect_play(clone!(
            #[weak(rename_to = obj)]
            self,
            move |_| obj.play()
        ));

        mpris_player.connect_pause(clone!(
            #[weak(rename_to = obj)]
            self,
            move |_| obj.pause()
        ));

        mpris_player.connect_play_pause(clone!(
            #[weak(rename_to = obj)]
            self,
            move |_| obj.play_pause()
        ));

        mpris_player.connect_previous(clone!(
            #[weak(rename_to = obj)]
            self,
            move |_| obj.previous()
        ));

        mpris_player.connect_next(clone!(
            #[weak(rename_to = obj)]
            self,
            move |_| obj.next()
        ));

        let task = mpris_player.run();

        self.imp()
            .mpris_player
            .set(mpris_player)
            .map_err(|_| anyhow!("Player already initialized"))?;

        Ok(task)
    }

    /// Generate new playlist items based on `program` and return the index of the first newly
    /// added item if successful.
    fn generate_items(&self, program: &Program) -> Result<u32> {
        let params = GenerateRecordingParams {
            composer_id: program.composer_id(),
            performer_id: program.performer_id(),
            ensemble_id: program.ensemble_id(),
            instrument_id: program.instrument_id(),
            work_id: program.work_id(),
            tag_id: program.tag_id(),
            tag_value: program.tag_value(),
            prefer_recently_added: program.prefer_recently_added(),
            prefer_least_recently_played: program.prefer_least_recently_played(),
            avoid_repeated_composers: program.avoid_repeated_composers(),
            avoid_repeated_instruments: program.avoid_repeated_instruments(),
        };

        let recording = self
            .library()
            .unwrap()
            .generate_recording(&params)
            .context("Failed to generate playlist items from program")?;

        // A program's `work_id` also matches recordings of an arrangement derived from
        // that work (see `candidates`), which is a different work entirely and has no
        // parts in common with it, so only scope down to specific tracks when the
        // chosen recording is actually hierarchically related to the target work.
        let target_work = program
            .work_id()
            .and_then(|work_id| self.library().unwrap().work(&work_id).ok())
            .filter(|work| {
                work.contains(&recording.work.work_id) || recording.work.contains(&work.work_id)
            });

        let playlist = match (target_work, program.play_full_recordings()) {
            (Some(work), true) => self.recording_to_playlist_for_work(&recording, &work),
            (Some(work), false) => {
                self.recording_to_playlist_for_work_single_track(&recording, &work)
            }
            (None, true) => self.recording_to_playlist(&recording),
            (None, false) => self.recording_to_playlist_single_track(&recording),
        };

        self.append(playlist)
    }

    fn library_path_to_file_path(&self, path: impl AsRef<Path>) -> String {
        PathBuf::from(self.library().unwrap().folder())
            .join(path)
            .to_str()
            .unwrap()
            .to_owned()
    }
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}
