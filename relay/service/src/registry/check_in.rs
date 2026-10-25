use crate::entity::RegistrationState;

pub struct CheckInRecord {
    pub name: String,
    pub state: RegistrationState,
    pub nonce: String,
    pub address_changed: bool,
}
