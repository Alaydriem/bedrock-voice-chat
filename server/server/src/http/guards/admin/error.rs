#[derive(Debug)]
pub enum AdminGuardError {
    MissingCertificate,
    PlayerNotFound,
    Banished,
    Forbidden,
    Internal,
}
