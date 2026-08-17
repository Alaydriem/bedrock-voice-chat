#[derive(Debug)]
pub enum PlayerGuardError {
    MissingCertificate,
    PlayerNotFound,
    Banished,
    Revoked,
    Internal,
}
