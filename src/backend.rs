use super::database::*;
use anyhow::Result;
use futures_channel::oneshot;
use futures_channel::oneshot::Sender;

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
    GetWorkDescriptions(i64, Sender<Result<Vec<WorkDescription>>>),
    UpdateEnsemble(Ensemble, Sender<Result<()>>),
    GetEnsemble(i64, Sender<Result<Ensemble>>),
    DeleteEnsemble(i64, Sender<Result<()>>),
    GetEnsembles(Sender<Result<Vec<Ensemble>>>),
    UpdateRecording(RecordingInsertion, Sender<Result<()>>),
    GetRecordingsForPerson(i64, Sender<Result<Vec<RecordingDescription>>>),
    GetRecordingsForEnsemble(i64, Sender<Result<Vec<RecordingDescription>>>),
    GetRecordingsForWork(i64, Sender<Result<Vec<RecordingDescription>>>),
}

use BackendAction::*;

pub struct Backend {
    action_sender: std::sync::mpsc::Sender<BackendAction>,
}

impl Backend {
    pub fn new(url: &str) -> Self {
        let url = url.to_string();

        let (action_sender, action_receiver) = std::sync::mpsc::channel::<BackendAction>();

        std::thread::spawn(move || {
            let db = Database::new(&url).expect("Failed to open database!");

            for action in action_receiver {
                match action {
                    UpdatePerson(person, sender) => {
                        db.update_person(person);
                        sender
                            .send(Ok(()))
                            .expect("Failed to send result from database thread!");
                    }
                    GetPerson(id, sender) => {
                        let person = db.get_person(id);
                        sender
                            .send(person)
                            .expect("Failed to send result from database thread!");
                    }
                    DeletePerson(id, sender) => {
                        db.delete_person(id);
                        sender
                            .send(Ok(()))
                            .expect("Failed to send result from database thread!");
                    }
                    GetPersons(sender) => {
                        let persons = db.get_persons();
                        sender
                            .send(persons)
                            .expect("Failed to send result from database thread!");
                    }
                    UpdateInstrument(instrument, sender) => {
                        db.update_instrument(instrument);
                        sender
                            .send(Ok(()))
                            .expect("Failed to send result from database thread!");
                    }
                    GetInstrument(id, sender) => {
                        let instrument = db.get_instrument(id);
                        sender
                            .send(instrument)
                            .expect("Failed to send result from database thread!");
                    }
                    DeleteInstrument(id, sender) => {
                        db.delete_instrument(id);
                        sender
                            .send(Ok(()))
                            .expect("Failed to send result from database thread!");
                    }
                    GetInstruments(sender) => {
                        let instruments = db.get_instruments();
                        sender
                            .send(instruments)
                            .expect("Failed to send result from database thread!");
                    }
                    UpdateWork(work, sender) => {
                        db.update_work(work);
                        sender
                            .send(Ok(()))
                            .expect("Failed to send result from database thread!");
                    }
                    GetWorkDescriptions(id, sender) => {
                        let works = db.get_work_descriptions(id);
                        sender
                            .send(works)
                            .expect("Failed to send result from database thread!");
                    }
                    UpdateEnsemble(ensemble, sender) => {
                        db.update_ensemble(ensemble);
                        sender
                            .send(Ok(()))
                            .expect("Failed to send result from database thread!");
                    }
                    GetEnsemble(id, sender) => {
                        let ensemble = db.get_ensemble(id);
                        sender
                            .send(ensemble)
                            .expect("Failed to send result from database thread!");
                    }
                    DeleteEnsemble(id, sender) => {
                        db.delete_ensemble(id);
                        sender
                            .send(Ok(()))
                            .expect("Failed to send result from database thread!");
                    }
                    GetEnsembles(sender) => {
                        let ensembles = db.get_ensembles();
                        sender
                            .send(ensembles)
                            .expect("Failed to send result from database thread!");
                    }
                    UpdateRecording(recording, sender) => {
                        db.update_recording(recording);
                        sender
                            .send(Ok(()))
                            .expect("Failed to send result from database thread!");
                    }
                    GetRecordingsForPerson(id, sender) => {
                        let recordings = db.get_recordings_for_person(id);
                        sender
                            .send(recordings)
                            .expect("Failed to send result from database thread!");
                    }
                    GetRecordingsForEnsemble(id, sender) => {
                        let recordings = db.get_recordings_for_ensemble(id);
                        sender
                            .send(recordings)
                            .expect("Failed to send result from database thread!");
                    }
                    GetRecordingsForWork(id, sender) => {
                        let recordings = db.get_recordings_for_work(id);
                        sender
                            .send(recordings)
                            .expect("Failed to send result from database thread!");
                    }
                }
            }
        });

        Backend {
            action_sender: action_sender,
        }
    }

    pub async fn update_person(&self, person: Person) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(UpdatePerson(person, sender));
        receiver.await?
    }

    pub async fn get_person(&self, id: i64) -> Result<Person> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(GetPerson(id, sender));
        receiver.await?
    }

    pub async fn delete_person(&self, id: i64) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(DeletePerson(id, sender));
        receiver.await?
    }

    pub async fn get_persons(&self) -> Result<Vec<Person>> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(GetPersons(sender));
        receiver.await?
    }

    pub async fn update_instrument(&self, instrument: Instrument) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender
            .send(UpdateInstrument(instrument, sender));
        receiver.await?
    }

    pub async fn get_instrument(&self, id: i64) -> Result<Instrument> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(GetInstrument(id, sender));
        receiver.await?
    }

    pub async fn delete_instrument(&self, id: i64) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(DeleteInstrument(id, sender));
        receiver.await?
    }

    pub async fn get_instruments(&self) -> Result<Vec<Instrument>> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(GetInstruments(sender));
        receiver.await?
    }

    pub async fn update_work(&self, work_insertion: WorkInsertion) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(UpdateWork(work_insertion, sender));
        receiver.await?
    }

    pub async fn get_work_descriptions(&self, person_id: i64) -> Result<Vec<WorkDescription>> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender
            .send(GetWorkDescriptions(person_id, sender));
        receiver.await?
    }

    pub async fn update_ensemble(&self, ensemble: Ensemble) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(UpdateEnsemble(ensemble, sender));
        receiver.await?
    }

    pub async fn get_ensemble(&self, id: i64) -> Result<Ensemble> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(GetEnsemble(id, sender));
        receiver.await?
    }

    pub async fn delete_ensemble(&self, id: i64) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(DeleteEnsemble(id, sender));
        receiver.await?
    }

    pub async fn get_ensembles(&self) -> Result<Vec<Ensemble>> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender.send(GetEnsembles(sender));
        receiver.await?
    }

    pub async fn update_recording(&self, recording_insertion: RecordingInsertion) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender
            .send(UpdateRecording(recording_insertion, sender));
        receiver.await?
    }

    pub async fn get_recordings_for_person(
        &self,
        person_id: i64,
    ) -> Result<Vec<RecordingDescription>> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender
            .send(GetRecordingsForPerson(person_id, sender));
        receiver.await?
    }

    pub async fn get_recordings_for_ensemble(
        &self,
        ensemble_id: i64,
    ) -> Result<Vec<RecordingDescription>> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender
            .send(GetRecordingsForEnsemble(ensemble_id, sender));
        receiver.await?
    }

    pub async fn get_recordings_for_work(&self, work_id: i64) -> Result<Vec<RecordingDescription>> {
        let (sender, receiver) = oneshot::channel();
        self.action_sender
            .send(GetRecordingsForWork(work_id, sender));
        receiver.await?
    }
}
