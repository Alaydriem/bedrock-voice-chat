use common::Game;

/// The authenticated identity a redeemed ticket confers on a socket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TicketIdentity {
    pub gamertag: String,
    pub game: Game,
}
