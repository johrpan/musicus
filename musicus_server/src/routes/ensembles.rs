use super::authenticate;
use crate::database;
use crate::database::{DbPool, Ensemble};
use crate::error::ServerError;
use actix_web::{delete, get, post, web, HttpResponse};
use actix_web_httpauth::extractors::bearer::BearerAuth;

/// Get an existing ensemble.
#[get("/ensembles/{id}")]
pub async fn get_ensemble(
    db: web::Data<DbPool>,
    id: web::Path<String>,
) -> Result<HttpResponse, ServerError> {
    let data = web::block(move || {
        let conn = db.into_inner().get()?;
        database::get_ensemble(&conn, &id.into_inner())?.ok_or(ServerError::NotFound)
    })
    .await?;

    Ok(HttpResponse::Ok().json(data))
}

/// Add a new ensemble or update an existin one. The user must be authorized to do that.
#[post("/ensembles")]
pub async fn update_ensemble(
    auth: BearerAuth,
    db: web::Data<DbPool>,
    data: web::Json<Ensemble>,
) -> Result<HttpResponse, ServerError> {
    web::block(move || {
        let conn = db.into_inner().get()?;
        let user = authenticate(&conn, auth.token()).or(Err(ServerError::Unauthorized))?;

        database::update_ensemble(&conn, &data.into_inner(), &user)?;

        Ok(())
    })
    .await?;

    Ok(HttpResponse::Ok().finish())
}

#[get("/ensembles")]
pub async fn get_ensembles(db: web::Data<DbPool>) -> Result<HttpResponse, ServerError> {
    let data = web::block(move || {
        let conn = db.into_inner().get()?;
        Ok(database::get_ensembles(&conn)?)
    })
    .await?;

    Ok(HttpResponse::Ok().json(data))
}

#[delete("/ensembles/{id}")]
pub async fn delete_ensemble(
    auth: BearerAuth,
    db: web::Data<DbPool>,
    id: web::Path<String>,
) -> Result<HttpResponse, ServerError> {
    web::block(move || {
        let conn = db.into_inner().get()?;
        let user = authenticate(&conn, auth.token()).or(Err(ServerError::Unauthorized))?;

        database::delete_ensemble(&conn, &id.into_inner(), &user)?;

        Ok(())
    })
    .await?;

    Ok(HttpResponse::Ok().finish())
}
