// Required for database/schema.rs
#[macro_use]
extern crate diesel;

// Required for embed_migrations macro in database/mod.rs
#[macro_use]
extern crate diesel_migrations;

use actix_web::{App, HttpServer};

mod database;
mod error;

mod routes;
use routes::*;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    sodiumoxide::init().expect("Failed to init crypto library!");
    let db_pool = database::connect().expect("Failed to create database interface!");

    let server = HttpServer::new(move || {
        App::new()
            .data(db_pool.clone())
            .wrap(actix_web::middleware::Logger::new(
                "%t: %r -> %s; %b B; %D ms",
            ))
            .service(register_user)
            .service(login_user)
            .service(put_user)
            .service(get_user)
            .service(get_person)
            .service(update_person)
            .service(get_persons)
            .service(delete_person)
            .service(get_ensemble)
            .service(update_ensemble)
            .service(delete_ensemble)
            .service(get_ensembles)
            .service(get_instrument)
            .service(update_instrument)
            .service(delete_instrument)
            .service(get_instruments)
            .service(get_work)
            .service(update_work)
            .service(delete_work)
            .service(get_works)
            .service(get_recording)
            .service(update_recording)
            .service(delete_recording)
            .service(get_recordings_for_work)
    });

    server.bind("127.0.0.1:8087")?.run().await
}
