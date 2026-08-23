use std::fs::OpenOptions;
use std::ops::RangeInclusive;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, SystemTime};

// Where an e2e server gets a port nothing else will take.
//
// Ports used to come from binding `127.0.0.1:0`, reading the assigned number and
// closing the socket. That number is then carried through the config into a
// server that binds it some milliseconds later, and the operating system hands a
// just-released port straight back out in the meantime — so two test processes
// racing through that window received the same one, and whichever server bound
// second died with `Only one usage of each socket address` (os error 10048). The
// test that owned it then failed on its boot timeout.
//
// Reserving instead of sampling closes both halves of that. The range sits
// outside the dynamic range the operating system allocates from, so no unrelated
// process is handed one of these; and a claim file makes the choice atomic
// between test processes, which are then the only competitors left.
pub struct PortPool;

impl PortPool {
    // Below the Windows dynamic range (49152+) and the Linux default ephemeral
    // range (32768+), and above the ports a developer machine typically serves
    // on. Nothing allocates from here unless it is asked to.
    pub const RANGE: RangeInclusive<u16> = 20_000..=39_999;

    // A claim only has to cover the gap between reserving a port and the server
    // binding it, which is milliseconds; the bind probe is what answers for a
    // port that is genuinely in use, so an expired claim on a live server is
    // still refused. Sized well above the 30s boot timeout all the same, and
    // short enough that a suite run does not consume the pool: a run claims a
    // couple of thousand ports, and nothing releases them until they age out.
    const STALE_AFTER: Duration = Duration::from_secs(60);

    /// Reserve a TCP port for this process.
    pub fn tcp() -> u16 {
        Self::reserve(|port| std::net::TcpListener::bind(("127.0.0.1", port)).is_ok())
    }

    /// Reserve a UDP port for this process.
    pub fn udp() -> u16 {
        Self::reserve(|port| std::net::UdpSocket::bind(("127.0.0.1", port)).is_ok())
    }

    /// Where the claim on `port` lives.
    pub fn claim_path(port: u16) -> PathBuf {
        std::env::temp_dir()
            .join("bvc-e2e-ports")
            .join(format!("{port}.claim"))
    }

    // Walks candidates until one is both unclaimed and free.
    //
    // The claim is taken before the bind probe, so a port that passes the probe
    // cannot be taken by another test process between the probe and the server's
    // own bind: the claim outlives this call, while the probe socket does not.
    fn reserve(is_free: impl Fn(u16) -> bool) -> u16 {
        Self::reclaim_stale_claims_once();

        let span = (Self::RANGE.end() - Self::RANGE.start()) as u32 + 1;

        for _ in 0..span {
            let offset = Self::cursor().fetch_add(1, Ordering::SeqCst) % span;
            let port = Self::RANGE.start() + offset as u16;

            if Self::claim(port) {
                if is_free(port) {
                    return port;
                }

                // Claimed but already bound by something outside this pool.
                // Holding the claim would retire that port for a whole minute
                // over a question the probe just answered.
                let _ = std::fs::remove_file(Self::claim_path(port));
            }
        }

        panic!(
            "no free port in {}..={}; {} claims are held",
            Self::RANGE.start(),
            Self::RANGE.end(),
            Self::held_claims()
        );
    }

    // Creating the file is the reservation: `create_new` fails if it exists, and
    // that check and the create are one operation in the filesystem, so two
    // processes cannot both believe they won.
    fn claim(port: u16) -> bool {
        let path = Self::claim_path(port);

        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .is_ok()
    }

    // Seeded from the process id so two test processes start far apart rather
    // than contending on the same candidates and falling through to the probe.
    fn cursor() -> &'static AtomicU32 {
        static CURSOR: std::sync::OnceLock<AtomicU32> = std::sync::OnceLock::new();
        CURSOR.get_or_init(|| {
            // Odd multiplier, so successive process ids scatter rather than
            // landing next to each other.
            AtomicU32::new(std::process::id().wrapping_mul(2_654_435_761))
        })
    }

    fn held_claims() -> usize {
        std::fs::read_dir(std::env::temp_dir().join("bvc-e2e-ports"))
            .map(|entries| entries.count())
            .unwrap_or(0)
    }

    // A claim leaks if the process holding it is killed, and a leaked claim
    // removes that port from the pool permanently. Reclaiming by age is safe
    // because a live claim is only ever seconds old.
    fn reclaim_stale_claims_once() {
        static RECLAIM: std::sync::Once = std::sync::Once::new();
        RECLAIM.call_once(|| {
            let dir = std::env::temp_dir().join("bvc-e2e-ports");
            let Ok(entries) = std::fs::read_dir(&dir) else {
                return;
            };

            for entry in entries.flatten() {
                let stale = entry
                    .metadata()
                    .and_then(|meta| meta.modified())
                    .map(|modified| {
                        SystemTime::now()
                            .duration_since(modified)
                            .map(|age| age > Self::STALE_AFTER)
                            .unwrap_or(false)
                    })
                    .unwrap_or(false);

                if stale {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        });
    }
}
