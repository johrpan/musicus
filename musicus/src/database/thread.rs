use super::*;
use anyhow::Result;
use futures_channel::oneshot;
use futures_channel::oneshot::Sender;
use std::sync::mpsc;
use std::thread;

/// An action the database thread can perform.
enum Action {
    UpdatePerson(Person, Sender<Result<()>>),
    GetPerson(String, Sender<Result<Option<Person>>>),
    DeletePerson(String, Sender<Result<()>>),
    GetPersons(Sender<Result<Vec<Person>>>),
    UpdateInstrument(Instrument, Sender<Result<()>>),
    GetInstrument(String, Sender<Result<Option<Instrument>>>),
    DeleteInstrument(String, Sender<Result<()>>),
    GetInstruments(Sender<Result<Vec<Instrument>>>),
    UpdateWork(Work, Sender<Result<()>>),
    DeleteWork(String, Sender<Result<()>>),
    GetWorks(String, Sender<Result<Vec<Work>>>),
    UpdateEnsemble(Ensemble, Sender<Result<()>>),
    GetEnsemble(String, Sender<Result<Option<Ensemble>>>),
    DeleteEnsemble(String, Sender<Result<()>>),
    GetEnsembles(Sender<Result<Vec<Ensemble>>>),
    UpdateRecording(Recording, Sender<Result<()>>),
    DeleteRecording(String, Sender<Result<()>>),
    GetRecordingsForPerson(String, Sender<Result<Vec<Recording>>>),
    GetRecordingsForEnsemble(String, Sender<Result<Vec<Recording>>>),
    GetRecordingsForWork(String, Sender<Result<Vec<Recording>>>),
    RecordingExists(String, Sender<Result<bool>>),
    UpdateMedium(Medium, Sender<Result<()>>),
    GetMedium(String, Sender<Result<Option<Medium>>>),
    DeleteMedium(String, Sender<Result<()>>),
    GetTracks(String, Sender<Result<Vec<Track>>>),
    Stop(Sender<()>),
}

use Action::*;

/// A database running within a thread.
pub struct DbThread {
    action_sender: mpsc::Sender<Action>,
}

impl DbThread {
    /// Create a new database connection in a background thread.
    pub async fn new(path: String) -> Result<Self> {
        let (action_sender, action_receiver) = mpsc::channel();
        let (ready_sender, ready_receiver) = oneshot::channel();

        thread::spawn(move || {
            let db = match Database::new(&path) {
                Ok(db) => {
                    ready_sender.send(Ok(())).unwrap();
                    db
                }
                Err(error) => {
                    ready_sender.send(Err(error)).unwrap();
                    return;
                }
            };

            for action in action_receiver {
                match action {
                    UpdatePerson(person, sender) => {
                        sender.send(db.update_person(person)).unwrap();
                    }
                    GetPerson(id, sender) => {
                        sender.send(db.get_person(&id)).unwrap();
                    }
                    DeletePerson(id, sender) => {
                        sender.send(db.delete_person(&id)).unwrap();
                    }
                    GetPersons(sender) => {
                        sender.send(db.get_persons()).unwrap();
                    }
                    UpdateInstrument(instrument, sender) => {
                        sender.send(db.update_instrument(instrument)).unwrap();
                    }
                    GetInstrument(id, sender) => {
                        sender.send(db.get_instrument(&id)).unwrap();
                    }
                    DeleteInstrument(id, sender) => {
                        sender.send(db.delete_instrument(&id)).unwrap();
                    }
                    GetInstruments(sender) => {
                        sender.send(db.get_instruments()).unwrap();
                    }
                    UpdateWork(work, sender) => {
                        sender.send(db.update_work(work)).unwrap();
                    }
                    DeleteWork(id, sender) => {
                        sender.send(db.delete_work(&id)).unwrap();
                    }
                    GetWorks(id, sender) => {
                        sender.send(db.get_works(&id)).unwrap();
                    }
                    UpdateEnsemble(ensemble, sender) => {
                        sender.send(db.update_ensemble(ensemble)).unwrap();
                    }
                    GetEnsemble(id, sender) => {
                        sender.send(db.get_ensemble(&id)).unwrap();
                    }
                    DeleteEnsemble(id, sender) => {
                        sender.send(db.delete_ensemble(&id)).unwrap();
                    }
                    GetEnsembles(sender) => {
                        sender.send(db.get_ensembles()).unwrap();
                    }
                    UpdateRecording(recording, sender) => {
                        sender.send(db.update_recording(recording)).unwrap();
                    }
                    DeleteRecording(id, sender) => {
                        sender.send(db.delete_recording(&id)).unwrap();
                    }
                    GetRecordingsForPerson(id, sender) => {
                        sender.send(db.get_recordings_for_person(&id)).unwrap();
                    }
                    GetRecordingsForEnsemble(id, sender) => {
                        sender.send(db.get_recordings_for_ensemble(&id)).unwrap();
                    }
                    GetRecordingsForWork(id, sender) => {
                        sender.send(db.get_recordings_for_work(&id)).unwrap();
                    }
                    RecordingExists(id, sender) => {
                        sender.send(db.recording_exists(&id)).unwrap();
                    }
                    UpdateMedium(medium, sender) => {
                        sender.send(db.update_medium(medium)).unwrap();
                    }
                    GetMedium(id, sender) => {
                        sender.send(db.get_medium(&id)).unwrap();
                    }
                    DeleteMedium(id, sender) => {
                        sender.send(db.delete_medium(&id)).unwrap();
                    }
                    GetTracks(recording_id, sender) => {
                        sender.send(db.get_tracks(&recording_id)).unwrap();
                    }
                    Stop(sender) => {
                        sender.send(()).unwrap();
                        break;
                    }
                }
            }
        });

        ready_receiver.await??;
        Ok(Self { action_sender })
    }

    /// Update an existing person or insert a new one.
    pub async fn update_person(&self, person: Person) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(UpdatePerson(person, sender))?;
        receiver.await?
    }

    /// Get an existing person.
    pub async fn get_person(&self, id: &str) -> Result<Option<Person>> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(GetPerson(id.to_string(), sender))?;
        receiver.await?
    }

    /// Delete an existing person. This will fail, if there are still other items referencing
    /// this person.
    pub async fn delete_person(&self, id: &str) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender
            .send(DeletePerson(id.to_string(), sender))?;
        receiver.await?
    }

    /// Get all existing persons.
    pub async fn get_persons(&self) -> Result<Vec<Person>> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(GetPersons(sender))?;
        receiver.await?
    }

    /// Update an existing instrument or insert a new one.
    pub async fn update_instrument(&self, instrument: Instrument) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender
            .send(UpdateInstrument(instrument, sender))?;
        receiver.await?
    }

    /// Get an existing instrument.
    pub async fn get_instrument(&self, id: &str) -> Result<Option<Instrument>> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender
            .send(GetInstrument(id.to_string(), sender))?;
        receiver.await?
    }

    /// Delete an existing instrument. This will fail, if there are still other items referencing
    /// this instrument.
    pub async fn delete_instrument(&self, id: &str) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender
            .send(DeleteInstrument(id.to_string(), sender))?;
        receiver.await?
    }

    /// Get all existing instruments.
    pub async fn get_instruments(&self) -> Result<Vec<Instrument>> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(GetInstruments(sender))?;
        receiver.await?
    }

    /// Update an existing work or insert a new one.
    pub async fn update_work(&self, work: Work) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(UpdateWork(work, sender))?;
        receiver.await?
    }

    /// Delete an existing work. This will fail, if there are still other items referencing
    /// this work.
    pub async fn delete_work(&self, id: &str) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender
            .send(DeleteWork(id.to_string(), sender))?;
        receiver.await?
    }

    /// Get information on all existing works by a composer.
    pub async fn get_works(&self, person_id: &str) -> Result<Vec<Work>> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender
            .send(GetWorks(person_id.to_string(), sender))?;
        receiver.await?
    }

    /// Update an existing ensemble or insert a new one.
    pub async fn update_ensemble(&self, ensemble: Ensemble) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(UpdateEnsemble(ensemble, sender))?;
        receiver.await?
    }

    /// Get an existing ensemble.
    pub async fn get_ensemble(&self, id: &str) -> Result<Option<Ensemble>> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender
            .send(GetEnsemble(id.to_string(), sender))?;
        receiver.await?
    }

    /// Delete an existing ensemble. This will fail, if there are still other items referencing
    /// this ensemble.
    pub async fn delete_ensemble(&self, id: &str) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender
            .send(DeleteEnsemble(id.to_string(), sender))?;
        receiver.await?
    }

    /// Get all existing ensembles.
    pub async fn get_ensembles(&self) -> Result<Vec<Ensemble>> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(GetEnsembles(sender))?;
        receiver.await?
    }

    /// Update an existing recording or insert a new one.
    pub async fn update_recording(&self, recording: Recording) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender
            .send(UpdateRecording(recording, sender))?;
        receiver.await?
    }

    /// Delete an existing recording.
    pub async fn delete_recording(&self, id: &str) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender
            .send(DeleteRecording(id.to_string(), sender))?;
        receiver.await?
    }

    /// Get information on all recordings in which a person performs.
    pub async fn get_recordings_for_person(&self, person_id: &str) -> Result<Vec<Recording>> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender
            .send(GetRecordingsForPerson(person_id.to_string(), sender))?;
        receiver.await?
    }

    /// Get information on all recordings in which an ensemble performs.
    pub async fn get_recordings_for_ensemble(&self, ensemble_id: &str) -> Result<Vec<Recording>> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender
            .send(GetRecordingsForEnsemble(ensemble_id.to_string(), sender))?;
        receiver.await?
    }

    /// Get information on all recordings of a work.
    pub async fn get_recordings_for_work(&self, work_id: &str) -> Result<Vec<Recording>> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender
            .send(GetRecordingsForWork(work_id.to_string(), sender))?;
        receiver.await?
    }

    /// Check whether a recording exists within the database.
    pub async fn recording_exists(&self, id: &str) -> Result<bool> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender
            .send(RecordingExists(id.to_string(), sender))?;
        receiver.await?
    }

    /// Update an existing medium or insert a new one.
    pub async fn update_medium(&self, medium: Medium) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(UpdateMedium(medium, sender))?;
        receiver.await?
    }

    /// Delete an existing medium. This will fail, if there are still other
    /// items referencing this medium.
    pub async fn delete_medium(&self, id: &str) -> Result<()> {
        let (sender, receiver) = oneshot::channel();

        self.action_sender
            .send(DeleteMedium(id.to_owned(), sender))?;

        receiver.await?
    }

    /// Get an existing medium.
    pub async fn get_medium(&self, id: &str) -> Result<Option<Medium>> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(GetMedium(id.to_owned(), sender))?;
        receiver.await?
    }

    /// Get all tracks for a recording.
    pub async fn get_tracks(&self, recording_id: &str) -> Result<Vec<Track>> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(GetTracks(recording_id.to_owned(), sender))?;
        receiver.await?
    }

    /// Stop the database thread. Any future access to the database will fail.
    pub async fn stop(&self) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(Stop(sender))?;
        Ok(receiver.await?)
    }
}
