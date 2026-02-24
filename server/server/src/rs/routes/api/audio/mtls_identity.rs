use rocket::{
    http::Status,
    mtls::Certificate,
    request::{FromRequest, Outcome, Request},
};

/// Request guard that requires a valid mTLS client certificate.
/// Returns 403 Forbidden instead of forwarding, which prevents Rocket's
/// "Request guard is forwarding" warning on unauthenticated requests.
pub struct MtlsIdentity<'r>(pub Certificate<'r>);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for MtlsIdentity<'r> {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match request.guard::<Certificate<'r>>().await {
            Outcome::Success(cert) => Outcome::Success(MtlsIdentity(cert)),
            _ => Outcome::Error((Status::Forbidden, ())),
        }
    }
}
