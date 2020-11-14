use super::secure;
use crate::database::*;
use crate::player::*;
use anyhow::{anyhow, Result};
use futures_channel::oneshot::Sender;
use futures_channel::{mpsc, oneshot};
use gio::prelude::*;
use serde::Serialize;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

/// Credentials used for login.
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LoginData {
    pub username: String,
    pub password: String,
}

pub enum BackendState {
    NoMusicLibrary,
    Loading,
    Ready,
}

enum BackendAction {
    UpdatePerson(Person, Sender<Result<()>>),
    GetPerson(i64, Sender<Result<Person>>),
    DeletePerson(i64, Sender<Result<()>>),
    GetPersons(Sender<Result<Vec<Person>>>),
    UpdateInstrument(Instrument, Sender<Result<()>>),
    GetInstrument(i64, Sender<Result<Instrument>>),
    DeleteInstrument(i64, Sender<Result<()>>),
    GetInstruments(Sender<Result<Vec<Instrument>>>),
    UpdateWork(WorkInsertion, Sender<Result<()>>),
    GetWorkDescription(i64, Sender<Result<WorkDescription>>),
    DeleteWork(i64, Sender<Result<()>>),
    GetWorkDescriptions(i64, Sender<Result<Vec<WorkDescription>>>),
    UpdateEnsemble(Ensemble, Sender<Result<()>>),
    GetEnsemble(i64, Sender<Result<Ensemble>>),
    DeleteEnsemble(i64, Sender<Result<()>>),
    GetEnsembles(Sender<Result<Vec<Ensemble>>>),
    UpdateRecording(RecordingInsertion, Sender<Result<()>>),
    GetRecordingDescription(i64, Sender<Result<RecordingDescription>>),
    DeleteRecording(i64, Sender<Result<()>>),
    GetRecordingsForPerson(i64, Sender<Result<Vec<RecordingDescription>>>),
    GetRecordingsForEnsemble(i64, Sender<Result<Vec<RecordingDescription>>>),
    GetRecordingsForWork(i64, Sender<Result<Vec<RecordingDescription>>>),
    UpdateTracks(i64, Vec<TrackDescription>, Sender<Result<()>>),
    DeleteTracks(i64, Sender<Result<()>>),
    GetTracks(i64, Sender<Result<Vec<TrackDescription>>>),
    Stop,
}

use BackendAction::*;

pub struct Backend {
    pub state_stream: RefCell<mpsc::Receiver<BackendState>>,
    state_sender: RefCell<mpsc::Sender<BackendState>>,
    action_sender: RefCell<Option<std::sync::mpsc::Sender<BackendAction>>>,
    settings: gio::Settings,
    secrets: secret_service::SecretService,
    server_url: RefCell<Option<String>>,
    login_data: RefCell<Option<LoginData>>,
    token: RefCell<Option<String>>,
    music_library_path: RefCell<Option<PathBuf>>,
    player: RefCell<Option<Rc<Player>>>,
}

impl Backend {
    pub fn new() -> Self {
        let (state_sender, state_stream) = mpsc::channel(1024);
        let secrets = secret_service::SecretService::new(secret_service::EncryptionType::Dh)
            .expect("Failed to connect to SecretsService!");

        Backend {
            state_stream: RefCell::new(state_stream),
            state_sender: RefCell::new(state_sender),
            action_sender: RefCell::new(None),
            settings: gio::Settings::new("de.johrpan.musicus"),
            secrets,
            music_library_path: RefCell::new(None),
            server_url: RefCell::new(None),
            login_data: RefCell::new(None),
            token: RefCell::new(None),
            player: RefCell::new(None),
        }
    }

    pub fn init(self: Rc<Backend>) {
        if let Some(path) = self.settings.get_string("music-library-path") {
            if !path.is_empty() {
                let context = glib::MainContext::default();
                let clone = self.clone();
                context.spawn_local(async move {
                    clone
                        .set_music_library_path_priv(PathBuf::from(path.to_string()))
                        .await
                        .unwrap();
                });
            }
        }

        if let Some(data) = secure::load_login_data().unwrap() {
            self.login_data.replace(Some(data));
        }

        if let Some(url) = self.settings.get_string("server-url") {
            if !url.is_empty() {
                self.server_url.replace(Some(url.to_string()));
            }
        }
    }

    pub async fn update_person(&self, person: Person) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.unwrap_action_sender()?
            .send(UpdatePerson(person, sender))?;
        receiver.await?
    }

    pub async fn get_person(&self, id: i64) -> Result<Person> {
        let (sender, receiver) = oneshot::channel();
        self.unwrap_action_sender()?.send(GetPerson(id, sender))?;
        receiver.await?
    }

    pub async fn delete_person(&self, id: i64) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.unwrap_action_sender()?
            .send(DeletePerson(id, sender))?;
        receiver.await?
    }

    pub async fn get_persons(&self) -> Result<Vec<Person>> {
        let (sender, receiver) = oneshot::channel();
        self.unwrap_action_sender()?.send(GetPersons(sender))?;
        receiver.await?
    }

    pub async fn update_instrument(&self, instrument: Instrument) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.unwrap_action_sender()?
            .send(UpdateInstrument(instrument, sender))?;
        receiver.await?
    }

    pub async fn get_instrument(&self, id: i64) -> Result<Instrument> {
        let (sender, receiver) = oneshot::channel();
        self.unwrap_action_sender()?
            .send(GetInstrument(id, sender))?;
        receiver.await?
    }

    pub async fn delete_instrument(&self, id: i64) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.unwrap_action_sender()?
            .send(DeleteInstrument(id, sender))?;
        receiver.await?
    }

    pub async fn get_instruments(&self) -> Result<Vec<Instrument>> {
        let (sender, receiver) = oneshot::channel();
        self.unwrap_action_sender()?.send(GetInstruments(sender))?;
        receiver.await?
    }

    pub async fn update_work(&self, work_insertion: WorkInsertion) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.unwrap_action_sender()?
            .send(UpdateWork(work_insertion, sender))?;
        receiver.await?
    }

    pub async fn get_work_description(&self, id: i64) -> Result<WorkDescription> {
        let (sender, receiver) = oneshot::channel();
        self.unwrap_action_sender()?
            .send(GetWorkDescription(id, sender))?;
        receiver.await?
    }

    pub async fn delete_work(&self, id: i64) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.unwrap_action_sender()?.send(DeleteWork(id, sender))?;
        receiver.await?
    }

    pub async fn get_work_descriptions(&self, person_id: i64) -> Result<Vec<WorkDescription>> {
        let (sender, receiver) = oneshot::channel();
        self.unwrap_action_sender()?
            .send(GetWorkDescriptions(person_id, sender))?;
        receiver.await?
    }

    pub async fn update_ensemble(&self, ensemble: Ensemble) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.unwrap_action_sender()?
            .send(UpdateEnsemble(ensemble, sender))?;
        receiver.await?
    }

    pub async fn get_ensemble(&self, id: i64) -> Result<Ensemble> {
        let (sender, receiver) = oneshot::channel();
        self.unwrap_action_sender()?.send(GetEnsemble(id, sender))?;
        receiver.await?
    }

    pub async fn delete_ensemble(&self, id: i64) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.unwrap_action_sender()?
            .send(DeleteEnsemble(id, sender))?;
        receiver.await?
    }

    pub async fn get_ensembles(&self) -> Result<Vec<Ensemble>> {
        let (sender, receiver) = oneshot::channel();
        self.unwrap_action_sender()?.send(GetEnsembles(sender))?;
        receiver.await?
    }

    pub async fn update_recording(&self, recording_insertion: RecordingInsertion) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.unwrap_action_sender()?
            .send(UpdateRecording(recording_insertion, sender))?;
        receiver.await?
    }

    pub async fn get_recording_description(&self, id: i64) -> Result<RecordingDescription> {
        let (sender, receiver) = oneshot::channel();
        self.unwrap_action_sender()?
            .send(GetRecordingDescription(id, sender))?;
        receiver.await?
    }

    pub async fn delete_recording(&self, id: i64) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.unwrap_action_sender()?
            .send(DeleteRecording(id, sender))?;
        receiver.await?
    }

    pub async fn get_recordings_for_person(
        &self,
        person_id: i64,
    ) -> Result<Vec<RecordingDescription>> {
        let (sender, receiver) = oneshot::channel();
        self.unwrap_action_sender()?
            .send(GetRecordingsForPerson(person_id, sender))?;
        receiver.await?
    }

    pub async fn get_recordings_for_ensemble(
        &self,
        ensemble_id: i64,
    ) -> Result<Vec<RecordingDescription>> {
        let (sender, receiver) = oneshot::channel();
        self.unwrap_action_sender()?
            .send(GetRecordingsForEnsemble(ensemble_id, sender))?;
        receiver.await?
    }

    pub async fn get_recordings_for_work(&self, work_id: i64) -> Result<Vec<RecordingDescription>> {
        let (sender, receiver) = oneshot::channel();
        self.unwrap_action_sender()?
            .send(GetRecordingsForWork(work_id, sender))?;
        receiver.await?
    }

    pub async fn update_tracks(
        &self,
        recording_id: i64,
        tracks: Vec<TrackDescription>,
    ) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.unwrap_action_sender()?
            .send(UpdateTracks(recording_id, tracks, sender))?;
        receiver.await?
    }

    pub async fn delete_tracks(&self, recording_id: i64) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.unwrap_action_sender()?
            .send(DeleteTracks(recording_id, sender))?;
        receiver.await?
    }

    pub async fn get_tracks(&self, recording_id: i64) -> Result<Vec<TrackDescription>> {
        let (sender, receiver) = oneshot::channel();
        self.unwrap_action_sender()?
            .send(GetTracks(recording_id, sender))?;
        receiver.await?
    }

    pub async fn set_music_library_path(&self, path: PathBuf) -> Result<()> {
        self.settings
            .set_string("music-library-path", path.to_str().unwrap())?;
        self.set_music_library_path_priv(path).await
    }

    pub fn get_music_library_path(&self) -> Option<PathBuf> {
        self.music_library_path.borrow().clone()
    }

    /// Get the currently stored login credentials.
    pub fn get_login_data(&self) -> Option<LoginData> {
        self.login_data.borrow().clone()
    }

    /// Set the URL of the Musicus server to connect to.
    pub fn set_server_url(&self, url: &str) -> Result<()> {
        self.settings.set_string("server-url", url)?;
        self.server_url.replace(Some(url.to_string()));
        Ok(())
    }

    /// Get the currently used login token.
    pub fn get_token(&self) -> Option<String> {
        self.token.borrow().clone()
    }

    /// Set the login token to use. This will be done automatically by the login method.
    pub fn set_token(&self, token: &str) {
        self.token.replace(Some(token.to_string()));
    }

    /// Get the currently set server URL.
    pub fn get_server_url(&self) -> Option<String> {
        self.server_url.borrow().clone()
    }

    /// Set the user credentials to use.
    pub async fn set_login_data(&self, data: LoginData) -> Result<()> {
        secure::store_login_data(data.clone()).await?;
        self.login_data.replace(Some(data));
        Ok(())
    }

    pub fn get_player(&self) -> Option<Rc<Player>> {
        self.player.borrow().clone()
    }

    async fn set_music_library_path_priv(&self, path: PathBuf) -> Result<()> {
        self.set_state(BackendState::Loading);

        if let Some(player) = &*self.player.borrow() {
            player.clear();
        }

        self.music_library_path.replace(Some(path.clone()));
        self.player.replace(Some(Player::new(path.clone())));

        if let Some(action_sender) = self.action_sender.borrow_mut().take() {
            action_sender.send(Stop)?;
        }

        let mut db_path = path.clone();
        db_path.push("musicus.db");

        self.start_db_thread(String::from(db_path.to_str().unwrap()))
            .await?;

        self.set_state(BackendState::Ready);

        Ok(())
    }

    fn set_state(&self, state: BackendState) {
        self.state_sender.borrow_mut().try_send(state).unwrap();
    }

    fn unwrap_action_sender(&self) -> Result<std::sync::mpsc::Sender<BackendAction>> {
        match &*self.action_sender.borrow() {
            Some(action_sender) => Ok(action_sender.clone()),
            None => Err(anyhow!("Database thread is not running!")),
        }
    }

    async fn start_db_thread(&self, url: String) -> Result<()> {
        let (ready_sender, ready_receiver) = oneshot::channel();
        let (action_sender, action_receiver) = std::sync::mpsc::channel::<BackendAction>();

        std::thread::spawn(move || {
            let db = Database::new(&url).expect("Failed to open database!");

            ready_sender
                .send(())
                .expect("Failed to communicate to main thread!");

            for action in action_receiver {
                match action {
                    UpdatePerson(person, sender) => {
                        sender
                            .send(db.update_person(person))
                            .expect("Failed to send result from database thread!");
                    }
                    GetPerson(id, sender) => {
                        sender
                            .send(db.get_person(id))
                            .expect("Failed to send result from database thread!");
                    }
                    DeletePerson(id, sender) => {
                        sender
                            .send(db.delete_person(id))
                            .expect("Failed to send result from database thread!");
                    }
                    GetPersons(sender) => {
                        sender
                            .send(db.get_persons())
                            .expect("Failed to send result from database thread!");
                    }
                    UpdateInstrument(instrument, sender) => {
                        sender
                            .send(db.update_instrument(instrument))
                            .expect("Failed to send result from database thread!");
                    }
                    GetInstrument(id, sender) => {
                        sender
                            .send(db.get_instrument(id))
                            .expect("Failed to send result from database thread!");
                    }
                    DeleteInstrument(id, sender) => {
                        sender
                            .send(db.delete_instrument(id))
                            .expect("Failed to send result from database thread!");
                    }
                    GetInstruments(sender) => {
                        sender
                            .send(db.get_instruments())
                            .expect("Failed to send result from database thread!");
                    }
                    UpdateWork(work, sender) => {
                        sender
                            .send(db.update_work(work))
                            .expect("Failed to send result from database thread!");
                    }
                    GetWorkDescription(id, sender) => {
                        sender
                            .send(db.get_work_description(id))
                            .expect("Failed to send result from database thread!");
                    }
                    DeleteWork(id, sender) => {
                        sender
                            .send(db.delete_work(id))
                            .expect("Failed to send result from database thread!");
                    }
                    GetWorkDescriptions(id, sender) => {
                        sender
                            .send(db.get_work_descriptions(id))
                            .expect("Failed to send result from database thread!");
                    }
                    UpdateEnsemble(ensemble, sender) => {
                        sender
                            .send(db.update_ensemble(ensemble))
                            .expect("Failed to send result from database thread!");
                    }
                    GetEnsemble(id, sender) => {
                        sender
                            .send(db.get_ensemble(id))
                            .expect("Failed to send result from database thread!");
                    }
                    DeleteEnsemble(id, sender) => {
                        sender
                            .send(db.delete_ensemble(id))
                            .expect("Failed to send result from database thread!");
                    }
                    GetEnsembles(sender) => {
                        sender
                            .send(db.get_ensembles())
                            .expect("Failed to send result from database thread!");
                    }
                    UpdateRecording(recording, sender) => {
                        sender
                            .send(db.update_recording(recording))
                            .expect("Failed to send result from database thread!");
                    }
                    GetRecordingDescription(id, sender) => {
                        sender
                            .send(db.get_recording_description(id))
                            .expect("Failed to send result from database thread!");
                    }
                    DeleteRecording(id, sender) => {
                        sender
                            .send(db.delete_recording(id))
                            .expect("Failed to send result from database thread!");
                    }
                    GetRecordingsForPerson(id, sender) => {
                        sender
                            .send(db.get_recordings_for_person(id))
                            .expect("Failed to send result from database thread!");
                    }
                    GetRecordingsForEnsemble(id, sender) => {
                        sender
                            .send(db.get_recordings_for_ensemble(id))
                            .expect("Failed to send result from database thread!");
                    }
                    GetRecordingsForWork(id, sender) => {
                        sender
                            .send(db.get_recordings_for_work(id))
                            .expect("Failed to send result from database thread!");
                    }
                    UpdateTracks(recording_id, tracks, sender) => {
                        sender
                            .send(db.update_tracks(recording_id, tracks))
                            .expect("Failed to send result from database thread!");
                    }
                    DeleteTracks(recording_id, sender) => {
                        sender
                            .send(db.delete_tracks(recording_id))
                            .expect("Failed to send result from database thread!");
                    }
                    GetTracks(recording_id, sender) => {
                        sender
                            .send(db.get_tracks(recording_id))
                            .expect("Failed to send result from database thread!");
                    }
                    Stop => {
                        break;
                    }
                }
            }
        });

        ready_receiver.await?;
        self.action_sender.replace(Some(action_sender));
        Ok(())
    }
}
