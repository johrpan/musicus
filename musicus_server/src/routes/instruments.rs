use super::authenticate;
use crate::database;
use crate::database::{DbPool, Instrument};
use crate::error::ServerError;
use actix_web::{delete, get, post, web, HttpResponse};
use actix_web_httpauth::extractors::bearer::BearerAuth;

/// Get an existing instrument.
#[get("/instruments/{id}")]
pub async fn get_instrument(
    db: web::Data<DbPool>,
    id: web::Path<String>,
) -> Result<HttpResponse, ServerError> {
    let data = web::block(move || {
        let conn = db.into_inner().get()?;
        database::get_instrument(&conn, &id.into_inner())?.ok_or(ServerError::NotFound)
    })
    .await?;

    Ok(HttpResponse::Ok().json(data))
}

/// Add a new instrument or update an existin one. The user must be authorized to do that.
#[post("/instruments")]
pub async fn update_instrument(
    auth: BearerAuth,
    db: web::Data<DbPool>,
    data: web::Json<Instrument>,
) -> Result<HttpResponse, ServerError> {
    web::block(move || {
        let conn = db.into_inner().get()?;
        let user = authenticate(&conn, auth.token()).or(Err(ServerError::Unauthorized))?;

        database::update_instrument(&conn, &data.into_inner(), &user)?;

        Ok(())
    })
    .await?;

    Ok(HttpResponse::Ok().finish())
}

#[get("/instruments")]
pub async fn get_instruments(db: web::Data<DbPool>) -> Result<HttpResponse, ServerError> {
    let data = web::block(move || {
        let conn = db.into_inner().get()?;
        Ok(database::get_instruments(&conn)?)
    })
    .await?;

    Ok(HttpResponse::Ok().json(data))
}

#[delete("/instruments/{id}")]
pub async fn delete_instrument(
    auth: BearerAuth,
    db: web::Data<DbPool>,
    id: web::Path<String>,
) -> Result<HttpResponse, ServerError> {
    web::block(move || {
        let conn = db.into_inner().get()?;
        let user = authenticate(&conn, auth.token()).or(Err(ServerError::Unauthorized))?;

        database::delete_instrument(&conn, &id.into_inner(), &user)?;

        Ok(())
    })
    .await?;

    Ok(HttpResponse::Ok().finish())
}
