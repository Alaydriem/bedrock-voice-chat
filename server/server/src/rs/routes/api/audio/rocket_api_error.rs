use rocket::http::Status;
use rocket::serde::json::Json;
use common::response::error::ApiError;

/// Rocket-specific wrapper that transposes `ApiError` into HTTP status + JSON body.
pub struct RocketApiError(pub ApiError);

impl<'r> rocket::response::Responder<'r, 'static> for RocketApiError {
    fn respond_to(self, req: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        let status = Status::new(self.0.status_code());
        (status, Json(self.0)).respond_to(req)
    }
}

impl From<ApiError> for RocketApiError {
    fn from(error: ApiError) -> Self {
        Self(error)
    }
}
