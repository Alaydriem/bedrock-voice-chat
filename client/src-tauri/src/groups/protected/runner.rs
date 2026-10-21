use std::time::Duration;

use tokio::sync::{mpsc, oneshot};

use super::{
    GroupKeyService, KeySocketEvent, ProtectedGroupCommand, ProtectedGroupEvent, SystemClock,
};

/// Runs one protected group: feeds key socket frames, people's decisions and a one-second tick to
/// the service, and passes each event it raises to `on_event`, until the group is lost or stopped.
pub struct ProtectedGroupRunner;

impl ProtectedGroupRunner {
    const TICK: Duration = Duration::from_secs(1);

    pub async fn run<F>(
        mut service: GroupKeyService<SystemClock>,
        mut socket_events: mpsc::Receiver<KeySocketEvent>,
        mut service_events: mpsc::Receiver<ProtectedGroupEvent>,
        mut commands: mpsc::Receiver<ProtectedGroupCommand>,
        mut stop: oneshot::Receiver<()>,
        mut on_event: F,
    ) where
        F: FnMut(ProtectedGroupEvent) + Send,
    {
        let mut tick = tokio::time::interval(Self::TICK);
        loop {
            let ended = tokio::select! {
                _ = &mut stop => {
                    service.leave();
                    true
                }
                event = socket_events.recv() => match event {
                    Some(KeySocketEvent::Frame(frame)) => {
                        service.on_frame(frame);
                        false
                    }
                    Some(KeySocketEvent::Closed(reason)) => {
                        service.on_socket_closed(reason.as_deref());
                        true
                    }
                    None => {
                        service.on_socket_closed(None);
                        true
                    }
                },
                Some(command) = commands.recv() => {
                    Self::apply(&mut service, command);
                    false
                }
                _ = tick.tick() => {
                    service.tick();
                    false
                }
            };

            let mut lost = false;
            while let Ok(event) = service_events.try_recv() {
                lost |= matches!(event, ProtectedGroupEvent::Lost(_));
                on_event(event);
            }
            if ended || lost {
                return;
            }
        }
    }

    fn apply(service: &mut GroupKeyService<SystemClock>, command: ProtectedGroupCommand) {
        match command {
            ProtectedGroupCommand::Check { joiner, reply } => {
                let _ = reply.send(service.check(joiner));
            }
            ProtectedGroupCommand::Decide {
                joiner,
                check_id,
                allow,
                reply,
            } => {
                let _ = reply.send(service.decide(joiner, check_id, allow));
            }
            ProtectedGroupCommand::Confirm {
                check_id,
                matches,
                reply,
            } => {
                let _ = reply.send(service.confirm(check_id, matches));
            }
        }
    }
}
