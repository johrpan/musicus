use super::database::*;
use glib::Sender;

enum BackendAction {
    UpdatePerson(Person, Sender<Result<(), String>>),
    GetPerson(i64, Sender<Option<Person>>),
    DeletePerson(i64, Sender<Result<(), String>>),
    GetPersons(Sender<Vec<Person>>),
    UpdateInstrument(Instrument, Sender<Result<(), String>>),
    GetInstrument(i64, Sender<Option<Instrument>>),
    DeleteInstrument(i64, Sender<Result<(), String>>),
    GetInstruments(Sender<Vec<Instrument>>),
    UpdateWork(WorkInsertion, Sender<Result<(), String>>),
    GetWorkDescriptions(i64, Sender<Vec<WorkDescription>>),
    UpdateEnsemble(Ensemble, Sender<Result<(), String>>),
    GetEnsemble(i64, Sender<Option<Ensemble>>),
    DeleteEnsemble(i64, Sender<Result<(), String>>),
    GetEnsembles(Sender<Vec<Ensemble>>),
    UpdateRecording(RecordingInsertion, Sender<Result<(), String>>),
    GetRecordingsForPerson(i64, Sender<Vec<RecordingDescription>>),
    GetRecordingsForEnsemble(i64, Sender<Vec<RecordingDescription>>),
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
            let db = Database::new(&url);

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
                }
            }
        });

        Backend {
            action_sender: action_sender,
        }
    }

    pub fn update_person<F: Fn(Result<(), String>) -> () + 'static>(
        &self,
        person: Person,
        callback: F,
    ) {
        let (sender, receiver) =
            glib::MainContext::channel::<Result<(), String>>(glib::PRIORITY_DEFAULT);

        receiver.attach(None, move |result| {
            callback(result);
            glib::Continue(true)
        });

        self.action_sender
            .send(UpdatePerson(person, sender))
            .expect("Failed to send action to database thread!");
    }

    pub fn get_person<F: Fn(Option<Person>) -> () + 'static>(&self, id: i64, callback: F) {
        let (sender, receiver) =
            glib::MainContext::channel::<Option<Person>>(glib::PRIORITY_DEFAULT);

        receiver.attach(None, move |result| {
            callback(result);
            glib::Continue(true)
        });

        self.action_sender
            .send(GetPerson(id, sender))
            .expect("Failed to send action to database thread!");
    }

    pub fn delete_person<F: Fn(Result<(), String>) -> () + 'static>(&self, id: i64, callback: F) {
        let (sender, receiver) =
            glib::MainContext::channel::<Result<(), String>>(glib::PRIORITY_DEFAULT);

        receiver.attach(None, move |result| {
            callback(result);
            glib::Continue(true)
        });

        self.action_sender
            .send(DeletePerson(id, sender))
            .expect("Failed to send action to database thread!");
    }

    pub fn get_persons<F: Fn(Vec<Person>) -> () + 'static>(&self, callback: F) {
        let (sender, receiver) = glib::MainContext::channel::<Vec<Person>>(glib::PRIORITY_DEFAULT);

        receiver.attach(None, move |result| {
            callback(result);
            glib::Continue(true)
        });

        self.action_sender
            .send(GetPersons(sender))
            .expect("Failed to send action to database thread!");
    }

    pub fn update_instrument<F: Fn(Result<(), String>) -> () + 'static>(
        &self,
        instrument: Instrument,
        callback: F,
    ) {
        let (sender, receiver) =
            glib::MainContext::channel::<Result<(), String>>(glib::PRIORITY_DEFAULT);

        receiver.attach(None, move |result| {
            callback(result);
            glib::Continue(true)
        });

        self.action_sender
            .send(UpdateInstrument(instrument, sender))
            .expect("Failed to send action to database thread!");
    }

    pub fn get_instrument<F: Fn(Option<Instrument>) -> () + 'static>(&self, id: i64, callback: F) {
        let (sender, receiver) =
            glib::MainContext::channel::<Option<Instrument>>(glib::PRIORITY_DEFAULT);

        receiver.attach(None, move |result| {
            callback(result);
            glib::Continue(true)
        });

        self.action_sender
            .send(GetInstrument(id, sender))
            .expect("Failed to send action to database thread!");
    }

    pub fn delete_instrument<F: Fn(Result<(), String>) -> () + 'static>(
        &self,
        id: i64,
        callback: F,
    ) {
        let (sender, receiver) =
            glib::MainContext::channel::<Result<(), String>>(glib::PRIORITY_DEFAULT);

        receiver.attach(None, move |result| {
            callback(result);
            glib::Continue(true)
        });

        self.action_sender
            .send(DeleteInstrument(id, sender))
            .expect("Failed to send action to database thread!");
    }

    pub fn get_instruments<F: Fn(Vec<Instrument>) -> () + 'static>(&self, callback: F) {
        let (sender, receiver) =
            glib::MainContext::channel::<Vec<Instrument>>(glib::PRIORITY_DEFAULT);

        receiver.attach(None, move |result| {
            callback(result);
            glib::Continue(true)
        });

        self.action_sender
            .send(GetInstruments(sender))
            .expect("Failed to send action to database thread!");
    }

    pub fn update_work<F: Fn(Result<(), String>) -> () + 'static>(
        &self,
        work: WorkInsertion,
        callback: F,
    ) {
        let (sender, receiver) =
            glib::MainContext::channel::<Result<(), String>>(glib::PRIORITY_DEFAULT);

        receiver.attach(None, move |result| {
            callback(result);
            glib::Continue(true)
        });

        self.action_sender
            .send(UpdateWork(work, sender))
            .expect("Failed to send action to database thread!");
    }

    pub fn get_work_descriptions<F: Fn(Vec<WorkDescription>) -> () + 'static>(
        &self,
        id: i64,
        callback: F,
    ) {
        let (sender, receiver) =
            glib::MainContext::channel::<Vec<WorkDescription>>(glib::PRIORITY_DEFAULT);

        receiver.attach(None, move |result| {
            callback(result);
            glib::Continue(true)
        });

        self.action_sender
            .send(GetWorkDescriptions(id, sender))
            .expect("Failed to send action to database thread!");
    }

    pub fn update_ensemble<F: Fn(Result<(), String>) -> () + 'static>(
        &self,
        ensemble: Ensemble,
        callback: F,
    ) {
        let (sender, receiver) =
            glib::MainContext::channel::<Result<(), String>>(glib::PRIORITY_DEFAULT);

        receiver.attach(None, move |result| {
            callback(result);
            glib::Continue(true)
        });

        self.action_sender
            .send(UpdateEnsemble(ensemble, sender))
            .expect("Failed to send action to database thread!");
    }

    pub fn get_ensemble<F: Fn(Option<Ensemble>) -> () + 'static>(&self, id: i64, callback: F) {
        let (sender, receiver) =
            glib::MainContext::channel::<Option<Ensemble>>(glib::PRIORITY_DEFAULT);

        receiver.attach(None, move |result| {
            callback(result);
            glib::Continue(true)
        });

        self.action_sender
            .send(GetEnsemble(id, sender))
            .expect("Failed to send action to database thread!");
    }

    pub fn delete_ensemble<F: Fn(Result<(), String>) -> () + 'static>(&self, id: i64, callback: F) {
        let (sender, receiver) =
            glib::MainContext::channel::<Result<(), String>>(glib::PRIORITY_DEFAULT);

        receiver.attach(None, move |result| {
            callback(result);
            glib::Continue(true)
        });

        self.action_sender
            .send(DeleteEnsemble(id, sender))
            .expect("Failed to send action to database thread!");
    }

    pub fn get_ensembles<F: Fn(Vec<Ensemble>) -> () + 'static>(&self, callback: F) {
        let (sender, receiver) =
            glib::MainContext::channel::<Vec<Ensemble>>(glib::PRIORITY_DEFAULT);

        receiver.attach(None, move |result| {
            callback(result);
            glib::Continue(true)
        });

        self.action_sender
            .send(GetEnsembles(sender))
            .expect("Failed to send action to database thread!");
    }

    pub fn update_recording<F: Fn(Result<(), String>) -> () + 'static>(
        &self,
        recording: RecordingInsertion,
        callback: F,
    ) {
        let (sender, receiver) =
            glib::MainContext::channel::<Result<(), String>>(glib::PRIORITY_DEFAULT);

        receiver.attach(None, move |result| {
            callback(result);
            glib::Continue(true)
        });

        self.action_sender
            .send(UpdateRecording(recording, sender))
            .expect("Failed to send action to database thread!");
    }

    pub fn get_recordings_for_person<F: Fn(Vec<RecordingDescription>) -> () + 'static>(
        &self,
        id: i64,
        callback: F,
    ) {
        let (sender, receiver) =
            glib::MainContext::channel::<Vec<RecordingDescription>>(glib::PRIORITY_DEFAULT);

        receiver.attach(None, move |result| {
            callback(result);
            glib::Continue(true)
        });

        self.action_sender
            .send(GetRecordingsForPerson(id, sender))
            .expect("Failed to send action to database thread!");
    }

    pub fn get_recordings_for_ensemble<F: Fn(Vec<RecordingDescription>) -> () + 'static>(
        &self,
        id: i64,
        callback: F,
    ) {
        let (sender, receiver) =
            glib::MainContext::channel::<Vec<RecordingDescription>>(glib::PRIORITY_DEFAULT);

        receiver.attach(None, move |result| {
            callback(result);
            glib::Continue(true)
        });

        self.action_sender
            .send(GetRecordingsForEnsemble(id, sender))
            .expect("Failed to send action to database thread!");
    }
}
