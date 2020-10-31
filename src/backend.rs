use super::database::*;
use anyhow::Result;
use futures_channel::oneshot;
use futures_channel::oneshot::Sender;
use std::cell::RefCell;
use std::path::PathBuf;

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
}

use BackendAction::*;

pub struct Backend {
    action_sender: std::sync::mpsc::Sender<BackendAction>,
    music_library_path: RefCell<Option<PathBuf>>,
}

impl Backend {
    pub fn new(url: &str, music_library_path: PathBuf) -> Self {
        let url = url.to_string();

        let (action_sender, action_receiver) = std::sync::mpsc::channel::<BackendAction>();

        std::thread::spawn(move || {
            let db = Database::new(&url).expect("Failed to open database!");

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
                }
            }
        });

        Backend {
            action_sender: action_sender,
            music_library_path: RefCell::new(Some(music_library_path)),
        }
    }

    pub async fn update_person(&self, person: Person) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(UpdatePerson(person, sender))?;
        receiver.await?
    }

    pub async fn get_person(&self, id: i64) -> Result<Person> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(GetPerson(id, sender))?;
        receiver.await?
    }

    pub async fn delete_person(&self, id: i64) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(DeletePerson(id, sender))?;
        receiver.await?
    }

    pub async fn get_persons(&self) -> Result<Vec<Person>> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(GetPersons(sender))?;
        receiver.await?
    }

    pub async fn update_instrument(&self, instrument: Instrument) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender
            .send(UpdateInstrument(instrument, sender))?;
        receiver.await?
    }

    pub async fn get_instrument(&self, id: i64) -> Result<Instrument> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(GetInstrument(id, sender))?;
        receiver.await?
    }

    pub async fn delete_instrument(&self, id: i64) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(DeleteInstrument(id, sender))?;
        receiver.await?
    }

    pub async fn get_instruments(&self) -> Result<Vec<Instrument>> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(GetInstruments(sender))?;
        receiver.await?
    }

    pub async fn update_work(&self, work_insertion: WorkInsertion) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender
            .send(UpdateWork(work_insertion, sender))?;
        receiver.await?
    }

    pub async fn get_work_description(&self, id: i64) -> Result<WorkDescription> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(GetWorkDescription(id, sender))?;
        receiver.await?
    }

    pub async fn delete_work(&self, id: i64) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(DeleteWork(id, sender))?;
        receiver.await?
    }

    pub async fn get_work_descriptions(&self, person_id: i64) -> Result<Vec<WorkDescription>> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender
            .send(GetWorkDescriptions(person_id, sender))?;
        receiver.await?
    }

    pub async fn update_ensemble(&self, ensemble: Ensemble) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(UpdateEnsemble(ensemble, sender))?;
        receiver.await?
    }

    pub async fn get_ensemble(&self, id: i64) -> Result<Ensemble> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(GetEnsemble(id, sender))?;
        receiver.await?
    }

    pub async fn delete_ensemble(&self, id: i64) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(DeleteEnsemble(id, sender))?;
        receiver.await?
    }

    pub async fn get_ensembles(&self) -> Result<Vec<Ensemble>> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(GetEnsembles(sender))?;
        receiver.await?
    }

    pub async fn update_recording(&self, recording_insertion: RecordingInsertion) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender
            .send(UpdateRecording(recording_insertion, sender))?;
        receiver.await?
    }

    pub async fn get_recording_description(&self, id: i64) -> Result<RecordingDescription> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender
            .send(GetRecordingDescription(id, sender))?;
        receiver.await?
    }

    pub async fn delete_recording(&self, id: i64) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(DeleteRecording(id, sender))?;
        receiver.await?
    }

    pub async fn get_recordings_for_person(
        &self,
        person_id: i64,
    ) -> Result<Vec<RecordingDescription>> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender
            .send(GetRecordingsForPerson(person_id, sender))?;
        receiver.await?
    }

    pub async fn get_recordings_for_ensemble(
        &self,
        ensemble_id: i64,
    ) -> Result<Vec<RecordingDescription>> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender
            .send(GetRecordingsForEnsemble(ensemble_id, sender))?;
        receiver.await?
    }

    pub async fn get_recordings_for_work(&self, work_id: i64) -> Result<Vec<RecordingDescription>> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender
            .send(GetRecordingsForWork(work_id, sender))?;
        receiver.await?
    }

    pub fn set_music_library_path(&self, path: &str) {
        self.music_library_path.replace(Some(PathBuf::from(path)));
    }

    pub fn get_music_library_path(&self) -> Option<PathBuf> {
        self.music_library_path.borrow().clone()
    }
}
