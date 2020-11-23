use super::{authenticate, may_create, may_delete, may_edit, ServerError};
use crate::database;
use crate::database::{DbPool, PersonInsertion};
use actix_web::{delete, get, post, put, web, HttpResponse};
use actix_web_httpauth::extractors::bearer::BearerAuth;

/// Get an existing person.
#[get("/persons/{id}")]
pub async fn get_person(
    db: web::Data<DbPool>,
    id: web::Path<u32>,
) -> Result<HttpResponse, ServerError> {
    let person = web::block(move || {
        let conn = db.into_inner().get()?;
        database::get_person(&conn, id.into_inner())?.ok_or(ServerError::NotFound)
    })
    .await?;

    Ok(HttpResponse::Ok().json(person))
}

/// Add a new person. The user must be authorized to do that.
#[post("/persons")]
pub async fn post_person(
    auth: BearerAuth,
    db: web::Data<DbPool>,
    data: web::Json<PersonInsertion>,
) -> Result<HttpResponse, ServerError> {
    let id = rand::random();

    web::block(move || {
        let conn = db.into_inner().get()?;
        let user = authenticate(&conn, auth.token()).or(Err(ServerError::Unauthorized))?;
        if may_create(&user) {
            database::insert_person(&conn, id, &data.into_inner(), &user.username)?;
            Ok(())
        } else {
            Err(ServerError::Forbidden)
        }
    })
    .await?;

    Ok(HttpResponse::Ok().body(id.to_string()))
}

#[put("/persons/{id}")]
pub async fn put_person(
    auth: BearerAuth,
    db: web::Data<DbPool>,
    id: web::Path<u32>,
    data: web::Json<PersonInsertion>,
) -> Result<HttpResponse, ServerError> {
    web::block(move || {
        let conn = db.into_inner().get()?;

        let user = authenticate(&conn, auth.token()).or(Err(ServerError::Unauthorized))?;

        let id = id.into_inner();
        let old_person = database::get_person(&conn, id)?.ok_or(ServerError::NotFound)?;

        if may_edit(&user, &old_person.created_by) {
            database::update_person(&conn, id, &data.into_inner())?;
            Ok(())
        } else {
            Err(ServerError::Forbidden)
        }
    })
    .await?;

    Ok(HttpResponse::Ok().finish())
}

#[get("/persons")]
pub async fn get_persons(db: web::Data<DbPool>) -> Result<HttpResponse, ServerError> {
    let persons = web::block(move || {
        let conn = db.into_inner().get()?;
        Ok(database::get_persons(&conn)?)
    })
    .await?;

    Ok(HttpResponse::Ok().json(persons))
}

#[delete("/persons/{id}")]
pub async fn delete_person(
    auth: BearerAuth,
    db: web::Data<DbPool>,
    id: web::Path<u32>,
) -> Result<HttpResponse, ServerError> {
    web::block(move || {
        let conn = db.into_inner().get()?;
        let user = authenticate(&conn, auth.token()).or(Err(ServerError::Unauthorized))?;

        if may_delete(&user) {
            database::delete_person(&conn, id.into_inner())?;
            Ok(())
        } else {
            Err(ServerError::Forbidden)
        }
    })
    .await?;

    Ok(HttpResponse::Ok().finish())
}
