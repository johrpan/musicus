use super::authenticate;
use crate::database;
use crate::database::{DbPool, Person};
use crate::error::ServerError;
use actix_web::{delete, get, post, web, HttpResponse};
use actix_web_httpauth::extractors::bearer::BearerAuth;

/// Get an existing person.
#[get("/persons/{id}")]
pub async fn get_person(
    db: web::Data<DbPool>,
    id: web::Path<String>,
) -> Result<HttpResponse, ServerError> {
    let data = web::block(move || {
        let conn = db.into_inner().get()?;
        database::get_person(&conn, &id.into_inner())?.ok_or(ServerError::NotFound)
    })
    .await?;

    Ok(HttpResponse::Ok().json(data))
}

/// Add a new person or update an existin one. The user must be authorized to do that.
#[post("/persons")]
pub async fn update_person(
    auth: BearerAuth,
    db: web::Data<DbPool>,
    data: web::Json<Person>,
) -> Result<HttpResponse, ServerError> {
    web::block(move || {
        let conn = db.into_inner().get()?;
        let user = authenticate(&conn, auth.token()).or(Err(ServerError::Unauthorized))?;

        database::update_person(&conn, &data.into_inner(), &user)?;

        Ok(())
    })
    .await?;

    Ok(HttpResponse::Ok().finish())
}

#[get("/persons")]
pub async fn get_persons(db: web::Data<DbPool>) -> Result<HttpResponse, ServerError> {
    let data = web::block(move || {
        let conn = db.into_inner().get()?;
        Ok(database::get_persons(&conn)?)
    })
    .await?;

    Ok(HttpResponse::Ok().json(data))
}

#[delete("/persons/{id}")]
pub async fn delete_person(
    auth: BearerAuth,
    db: web::Data<DbPool>,
    id: web::Path<String>,
) -> Result<HttpResponse, ServerError> {
    web::block(move || {
        let conn = db.into_inner().get()?;
        let user = authenticate(&conn, auth.token()).or(Err(ServerError::Unauthorized))?;

        database::delete_person(&conn, &id.into_inner(), &user)?;

        Ok(())
    })
    .await?;

    Ok(HttpResponse::Ok().finish())
}
