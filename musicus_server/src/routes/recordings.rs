use super::authenticate;
use crate::database;
use crate::database::{DbPool, Recording};
use crate::error::ServerError;
use actix_web::{delete, get, post, web, HttpResponse};
use actix_web_httpauth::extractors::bearer::BearerAuth;

/// Get an existing recording.
#[get("/recordings/{id}")]
pub async fn get_recording(
    db: web::Data<DbPool>,
    id: web::Path<String>,
) -> Result<HttpResponse, ServerError> {
    let data = web::block(move || {
        let conn = db.into_inner().get()?;
        database::get_recording(&conn, &id.into_inner())?.ok_or(ServerError::NotFound)
    })
    .await?;

    Ok(HttpResponse::Ok().json(data))
}

/// Add a new recording or update an existin one. The user must be authorized to do that.
#[post("/recordings")]
pub async fn update_recording(
    auth: BearerAuth,
    db: web::Data<DbPool>,
    data: web::Json<Recording>,
) -> Result<HttpResponse, ServerError> {
    web::block(move || {
        let conn = db.into_inner().get()?;
        let user = authenticate(&conn, auth.token()).or(Err(ServerError::Unauthorized))?;

        database::update_recording(&conn, &data.into_inner(), &user)?;

        Ok(())
    })
    .await?;

    Ok(HttpResponse::Ok().finish())
}

#[get("/works/{id}/recordings")]
pub async fn get_recordings_for_work(
    db: web::Data<DbPool>,
    work_id: web::Path<String>,
) -> Result<HttpResponse, ServerError> {
    let data = web::block(move || {
        let conn = db.into_inner().get()?;
        Ok(database::get_recordings_for_work(&conn, &work_id.into_inner())?)
    })
    .await?;

    Ok(HttpResponse::Ok().json(data))
}

#[delete("/recordings/{id}")]
pub async fn delete_recording(
    auth: BearerAuth,
    db: web::Data<DbPool>,
    id: web::Path<String>,
) -> Result<HttpResponse, ServerError> {
    web::block(move || {
        let conn = db.into_inner().get()?;
        let user = authenticate(&conn, auth.token()).or(Err(ServerError::Unauthorized))?;

        database::delete_recording(&conn, &id.into_inner(), &user)?;

        Ok(())
    })
    .await?;

    Ok(HttpResponse::Ok().finish())
}
