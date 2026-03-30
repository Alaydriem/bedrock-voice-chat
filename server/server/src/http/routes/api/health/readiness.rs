use rocket::http::Status;
use rocket::State;
use rocket_okapi::openapi;
use sea_orm::DatabaseConnection;

use crate::http::pool::Db;

#[openapi(tag = "Health")]
#[get("/readiness")]
pub async fn readiness(db: Db<'_>) -> Status {
    let conn: &DatabaseConnection = db.into_inner();
    match conn.ping().await {
        Ok(_) => Status::Ok,
        Err(_) => Status::ServiceUnavailable,
    }
}
