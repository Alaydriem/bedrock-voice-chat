//! End-to-end encrypted groups. Everything here runs on the client only: the server relays the
//! messages it produces and the frames it seals, and holds none of this code.

mod admission;
mod command;
mod gate_state;
mod key_socket;
mod key_socket_event;
mod mls;
mod notifier;
mod runner;
mod seal_gate;
mod service;
mod sframe;
mod slot;

pub use self::admission::{
    AdmissionRateLimiter, ApprovalCode, ApprovalCommitment, CheckError, CheckFields, CheckIdHex,
    ConfirmOutcome, DecideError, JoinerRequest, MemberCheck, QueuedAdd, SeenKeyPackages,
    WelcomeRefusal,
};
pub use self::command::ProtectedGroupCommand;
pub use self::key_socket::KeySocket;
pub use self::key_socket_event::KeySocketEvent;
pub use self::mls::{MlsGroupState, PendingJoin, RosterMember, SafetyCode, StagedAdd, StagedJoin};
pub use self::notifier::ProtectedGroupNotifier;
pub use self::runner::ProtectedGroupRunner;
pub use self::seal_gate::SealGate;
pub use self::slot::ProtectedGroupSlot;
pub use self::service::{
    Clock, CodeRole, CommitterRank, GroupKeyService, LostReason, ProtectedGroupEvent, SystemClock,
};
pub use self::sframe::{OpenedFrame, SframeCipher};

use gate_state::GateState;
