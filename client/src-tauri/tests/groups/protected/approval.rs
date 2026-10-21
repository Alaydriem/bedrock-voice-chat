use std::time::{Duration, Instant};

use bvc_client_lib::groups::protected::{
    AdmissionRateLimiter, ApprovalCode, ApprovalCommitment, CheckError, CheckFields, CheckIdHex,
    ConfirmOutcome, DecideError, JoinerRequest, MemberCheck, MlsGroupState, QueuedAdd,
    SeenKeyPackages, WelcomeRefusal,
};
use bytes::Bytes;
use common::structs::group_key::{AdmissionMessage, SocketId};

const GID: &str = "NR4K-2PQX";
const JOINER: SocketId = 7;
const MEMBER: SocketId = 3;

fn identity(name: &str) -> common::PlayerIdentity {
    format!("minecraft:{name}").parse().unwrap()
}

fn key_package(name: &str) -> Vec<u8> {
    MlsGroupState::key_package(&identity(name)).unwrap().1
}

struct Member {
    signature_key: Vec<u8>,
    mls_group_id: Vec<u8>,
    _state: MlsGroupState,
}

fn member() -> Member {
    let state = MlsGroupState::create(&identity("Alice")).unwrap();
    Member {
        signature_key: state.own_signature_key(),
        mls_group_id: state.mls_group_id(),
        _state: state,
    }
}

fn request(gid: &str, key_package: Vec<u8>, now: Instant) -> (JoinerRequest, [u8; 32]) {
    let (request, message) = JoinerRequest::new(gid.to_string(), key_package, now).unwrap();
    let AdmissionMessage::JoinRequest { commitment } = message else {
        panic!("expected a JoinRequest, got {message:?}");
    };
    (request, commitment)
}

fn check_fields(message: &AdmissionMessage) -> CheckFields {
    let AdmissionMessage::Check {
        check_id,
        nonce,
        signature_key,
        mls_group_id,
        member,
    } = message
    else {
        panic!("expected a Check, got {message:?}");
    };
    CheckFields {
        check_id: *check_id,
        nonce: *nonce,
        signature_key: signature_key.clone(),
        mls_group_id: mls_group_id.clone(),
        member: member.clone(),
    }
}

fn reveal_parts(message: &AdmissionMessage) -> ([u8; 16], Bytes, [u8; 32]) {
    let AdmissionMessage::Reveal {
        check_id,
        key_package,
        nonce,
    } = message
    else {
        panic!("expected a Reveal, got {message:?}");
    };
    (*check_id, key_package.clone(), *nonce)
}

/// Runs a request, a check and a reveal, and returns both sides with both codes.
struct Exchange {
    joiner: JoinerRequest,
    check: MemberCheck,
    joiner_code: ApprovalCode,
    member_code: Result<ApprovalCode, CheckError>,
    check_id: [u8; 16],
    member: Member,
    seen: SeenKeyPackages,
}

fn exchange(now: Instant) -> Exchange {
    let member = member();
    let (mut joiner, commitment) = request(GID, key_package("Steve"), now);
    let (mut check, message) = MemberCheck::start(
        GID.to_string(),
        JOINER,
        commitment,
        member.signature_key.clone(),
        member.mls_group_id.clone(),
        "minecraft:Alice".to_string(),
        now,
    )
    .unwrap();
    let fields = check_fields(&message);
    let (reveal, joiner_code) = joiner.on_check(MEMBER, fields, now).unwrap();
    let (check_id, key_package, nonce) = reveal_parts(&reveal);
    let mut seen = SeenKeyPackages::new();
    let member_code = check.on_reveal(
        JOINER,
        check_id,
        &key_package,
        &nonce,
        "minecraft:Steve",
        &mut seen,
        now,
    );
    Exchange {
        joiner,
        check,
        joiner_code,
        member_code,
        check_id,
        member,
        seen,
    }
}

// The code is a cross-version contract: two app versions must compute the same digits.
#[test]
fn code_matches_the_fixed_vector() {
    let code = ApprovalCode::derive(GID, &[1u8; 64], &[2; 32], &[3; 32], &[4u8; 32], &[5u8; 16])
        .unwrap();

    assert_eq!(code.as_digits(), "85033094");
    assert_eq!(code.display(), "8503 3094");
}

#[test]
fn commitment_matches_the_fixed_vector() {
    let commitment = ApprovalCommitment::of(GID, &[1u8; 64], &[2; 32]).unwrap();

    assert_eq!(
        commitment.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        "d7545545aa9f3a428cce171a26d34fe62a97db9414b2f956d056eae5cd183b95"
    );
}

#[test]
fn the_group_id_is_read_upper_case() {
    assert_eq!(
        ApprovalCommitment::of("nr4k-2pqx", &[1u8; 64], &[2; 32]),
        ApprovalCommitment::of(GID, &[1u8; 64], &[2; 32])
    );
}

#[test]
fn joiner_and_member_compute_the_same_code() {
    let exchange = exchange(Instant::now());

    assert_eq!(exchange.member_code, Ok(exchange.joiner_code));
}

#[test]
fn request_for_group_a_at_a_group_b_member_is_a_commitment_mismatch() {
    let now = Instant::now();
    let member = member();
    let (mut joiner, commitment) = request("AAAA-AAAA", key_package("Steve"), now);
    let (mut check, message) = MemberCheck::start(
        "BBBB-BBBB".to_string(),
        JOINER,
        commitment,
        member.signature_key.clone(),
        member.mls_group_id.clone(),
        "minecraft:Alice".to_string(),
        now,
    )
    .unwrap();
    let (reveal, _) = joiner.on_check(MEMBER, check_fields(&message), now).unwrap();
    let (check_id, key_package, nonce) = reveal_parts(&reveal);

    let result = check.on_reveal(
        JOINER,
        check_id,
        &key_package,
        &nonce,
        "minecraft:Steve",
        &mut SeenKeyPackages::new(),
        now,
    );

    assert_eq!(result, Err(CheckError::CommitmentMismatch));
}

// A server that collected a reveal must not get a second opening of the same commitment, however
// long it waits: that is what stops it grinding codes offline.
#[test]
fn second_check_after_a_reveal_is_busy_even_after_expiry() {
    let now = Instant::now();
    let member = member();
    let mut exchange = exchange(now);
    let (_second, message) = MemberCheck::start(
        GID.to_string(),
        JOINER,
        [9; 32],
        member.signature_key.clone(),
        member.mls_group_id.clone(),
        "minecraft:Alice".to_string(),
        now,
    )
    .unwrap();
    let second = check_fields(&message);
    let second_id = second.check_id;

    let later = now + Duration::from_secs(500);
    let answer = exchange.joiner.on_check(MEMBER + 1, second, later);

    assert_eq!(
        answer.map(|(message, _)| message),
        Err(AdmissionMessage::Busy {
            check_id: second_id
        })
    );
}

#[test]
fn ended_check_ends_the_request_and_the_next_request_has_a_new_commitment() {
    let now = Instant::now();
    let mut exchange = exchange(now);

    assert_eq!(
        exchange.joiner.confirm(exchange.check_id, false),
        ConfirmOutcome::Mismatch
    );
    assert!(exchange.joiner.is_ended(now));

    let package = key_package("Steve");
    let (_first, first) = request(GID, package.clone(), now);
    let (_second, second) = request(GID, package, now);
    assert_ne!(first, second, "a new request draws a new nonce");
}

#[test]
fn welcome_before_it_matches_is_not_accepted() {
    let exchange = exchange(Instant::now());

    let result = exchange.joiner.accept_welcome(
        exchange.check_id,
        MEMBER,
        &exchange.member.signature_key,
        &exchange.member.mls_group_id,
    );

    assert_eq!(result, Err(WelcomeRefusal::NotConfirmed));
}

// The server can send its own Check and then a Welcome signed with that key. Without the
// joiner's confirmation it never joins.
#[test]
fn welcome_signed_by_the_check_key_without_confirmation_does_not_join() {
    let now = Instant::now();
    let mut exchange = exchange(now);

    assert_eq!(
        exchange.joiner.accept_welcome(
            exchange.check_id,
            MEMBER,
            &exchange.member.signature_key,
            &exchange.member.mls_group_id,
        ),
        Err(WelcomeRefusal::NotConfirmed)
    );
    assert_eq!(
        exchange.joiner.confirm(exchange.check_id, true),
        ConfirmOutcome::Confirmed
    );
    assert_eq!(
        exchange.joiner.accept_welcome(
            exchange.check_id,
            MEMBER,
            &exchange.member.signature_key,
            &exchange.member.mls_group_id,
        ),
        Ok(())
    );
}

#[test]
fn welcome_from_another_signer_or_another_mls_group_is_refused() {
    let now = Instant::now();
    let mut exchange = exchange(now);
    exchange.joiner.confirm(exchange.check_id, true);
    let stranger = member();

    assert_eq!(
        exchange.joiner.accept_welcome(
            exchange.check_id,
            MEMBER,
            &stranger.signature_key,
            &exchange.member.mls_group_id,
        ),
        Err(WelcomeRefusal::WrongSigner)
    );
    assert_eq!(
        exchange.joiner.accept_welcome(
            exchange.check_id,
            MEMBER,
            &exchange.member.signature_key,
            &stranger.mls_group_id,
        ),
        Err(WelcomeRefusal::WrongGroup)
    );
    assert_eq!(
        exchange.joiner.accept_welcome(
            [0; 16],
            MEMBER,
            &exchange.member.signature_key,
            &exchange.member.mls_group_id,
        ),
        Err(WelcomeRefusal::WrongCheck)
    );
}

#[test]
fn decide_with_a_stale_check_id_is_refused() {
    let now = Instant::now();
    let exchange = exchange(now);

    let result = exchange.check.decide([0; 16], now);

    assert_eq!(result.err(), Some(DecideError::StaleCheck));
}

#[test]
fn decide_returns_the_stored_key_package_for_its_check() {
    let now = Instant::now();
    let exchange = exchange(now);
    let check_id = exchange.check_id;

    let queued = exchange.check.decide(check_id, now).unwrap();

    assert_eq!(queued.check_id, check_id);
    assert_eq!(queued.joiner, JOINER);
    assert_eq!(
        MlsGroupState::key_package_identity(&queued.key_package).map(|i| i.to_string()),
        Some("minecraft:Steve".to_string())
    );
}

#[test]
fn decide_after_the_check_expires_is_refused() {
    let now = Instant::now();
    let exchange = exchange(now);

    let result = exchange
        .check
        .decide(exchange.check_id, now + MemberCheck::LIFETIME);

    assert_eq!(result.err(), Some(DecideError::Expired));
}

// A stray frame from another socket must not use up the one reveal the check accepts.
#[test]
fn reveal_from_another_socket_is_wrong_sender_and_the_real_reveal_still_works() {
    let now = Instant::now();
    let member = member();
    let (mut joiner, commitment) = request(GID, key_package("Steve"), now);
    let (mut check, message) = MemberCheck::start(
        GID.to_string(),
        JOINER,
        commitment,
        member.signature_key.clone(),
        member.mls_group_id.clone(),
        "minecraft:Alice".to_string(),
        now,
    )
    .unwrap();
    let (reveal, joiner_code) = joiner.on_check(MEMBER, check_fields(&message), now).unwrap();
    let (check_id, key_package, nonce) = reveal_parts(&reveal);
    let mut seen = SeenKeyPackages::new();

    let stray = check.on_reveal(
        JOINER + 1,
        check_id,
        &key_package,
        &nonce,
        "minecraft:Steve",
        &mut seen,
        now,
    );
    let real = check.on_reveal(
        JOINER,
        check_id,
        &key_package,
        &nonce,
        "minecraft:Steve",
        &mut seen,
        now,
    );

    assert_eq!(stray, Err(CheckError::WrongSender));
    assert_eq!(real, Ok(joiner_code));
}

#[test]
fn a_key_package_naming_someone_else_is_an_identity_mismatch() {
    let now = Instant::now();
    let member = member();
    let (mut joiner, commitment) = request(GID, key_package("Mallory"), now);
    let (mut check, message) = MemberCheck::start(
        GID.to_string(),
        JOINER,
        commitment,
        member.signature_key.clone(),
        member.mls_group_id.clone(),
        "minecraft:Alice".to_string(),
        now,
    )
    .unwrap();
    let (reveal, _) = joiner.on_check(MEMBER, check_fields(&message), now).unwrap();
    let (check_id, key_package, nonce) = reveal_parts(&reveal);

    let result = check.on_reveal(
        JOINER,
        check_id,
        &key_package,
        &nonce,
        "minecraft:Steve",
        &mut SeenKeyPackages::new(),
        now,
    );

    assert_eq!(result, Err(CheckError::IdentityMismatch));
}

#[test]
fn joiner_after_it_matches_waits_until_120_s_not_60_s_after_reveal() {
    let now = Instant::now();
    let mut unconfirmed = exchange(now);
    let mut confirmed = exchange(now);
    confirmed.joiner.confirm(confirmed.check_id, true);

    let after_check = now + JoinerRequest::CHECK_LIFETIME;
    assert!(unconfirmed.joiner.is_ended(after_check));
    assert!(!confirmed.joiner.is_ended(after_check));
    assert!(confirmed.joiner.is_ended(now + JoinerRequest::LIFETIME));
    assert_eq!(
        unconfirmed.joiner.confirm(unconfirmed.check_id, true),
        ConfirmOutcome::Confirmed,
        "confirming is still recorded; expiry is the timer's decision"
    );
}

#[test]
fn queued_add_is_too_old_after_90_s() {
    let now = Instant::now();
    let queued = QueuedAdd {
        check_id: [1; 16],
        joiner: JOINER,
        key_package: Vec::new(),
        checked_at: now,
    };

    assert!(!queued.is_too_old(now + Duration::from_secs(89)));
    assert!(queued.is_too_old(now + QueuedAdd::MAX_AGE));
}

#[test]
fn repeated_key_package_is_refused() {
    let now = Instant::now();
    let mut first = exchange(now);
    assert!(first.member_code.is_ok());
    let member = member();
    let replayed = first.check.decide(first.check_id, now).unwrap().key_package;
    let (mut joiner, commitment) = request(GID, replayed, now);
    let (mut check, message) = MemberCheck::start(
        GID.to_string(),
        JOINER,
        commitment,
        member.signature_key.clone(),
        member.mls_group_id.clone(),
        "minecraft:Alice".to_string(),
        now,
    )
    .unwrap();
    let (reveal, _) = joiner.on_check(MEMBER, check_fields(&message), now).unwrap();
    let (check_id, key_package, nonce) = reveal_parts(&reveal);

    let result = check.on_reveal(
        JOINER,
        check_id,
        &key_package,
        &nonce,
        "minecraft:Steve",
        &mut first.seen,
        now,
    );

    assert_eq!(result, Err(CheckError::Replayed));
}

#[test]
fn invalid_key_package_is_refused_before_a_code() {
    let now = Instant::now();
    let member = member();
    let (mut joiner, commitment) = request(GID, vec![0xAB; 80], now);
    let (mut check, message) = MemberCheck::start(
        GID.to_string(),
        JOINER,
        commitment,
        member.signature_key.clone(),
        member.mls_group_id.clone(),
        "minecraft:Alice".to_string(),
        now,
    )
    .unwrap();
    let (reveal, _) = joiner.on_check(MEMBER, check_fields(&message), now).unwrap();
    let (check_id, key_package, nonce) = reveal_parts(&reveal);

    let result = check.on_reveal(
        JOINER,
        check_id,
        &key_package,
        &nonce,
        "minecraft:Steve",
        &mut SeenKeyPackages::new(),
        now,
    );

    assert_eq!(result, Err(CheckError::InvalidKeyPackage));
    assert_eq!(check.code(), None);
}

#[test]
fn limiter_stops_a_fourth_check_on_the_same_identity_in_ten_minutes() {
    let now = Instant::now();
    let mut limiter = AdmissionRateLimiter::new();

    for _ in 0..3 {
        assert!(limiter.permits_check("minecraft:Steve", now));
        limiter.record_failure("minecraft:Steve", now);
    }

    assert!(!limiter.permits_check("minecraft:Steve", now));
    assert!(limiter.permits_check("minecraft:Alex", now));
    assert!(limiter.permits_check("minecraft:Steve", now + Duration::from_secs(10 * 60)));
}

#[test]
fn a_check_id_crosses_the_webview_as_32_hex_characters_and_nothing_else_parses() {
    let id = [0xAB; 16];

    assert_eq!(CheckIdHex::decode(&CheckIdHex::encode(&id)), Some(id));
    assert_eq!(CheckIdHex::decode("ab"), None);
    assert_eq!(CheckIdHex::decode(&"zz".repeat(16)), None);
    assert_eq!(CheckIdHex::decode(&"ab".repeat(17)), None);
}

#[test]
fn limiter_stops_an_eleventh_failure_in_the_group_within_an_hour() {
    let now = Instant::now();
    let mut limiter = AdmissionRateLimiter::new();

    for n in 0..10 {
        limiter.record_failure(&format!("minecraft:P{n}"), now);
    }

    assert!(!limiter.permits_check("minecraft:Fresh", now + Duration::from_secs(59 * 60)));
    assert!(limiter.permits_check("minecraft:Fresh", now + Duration::from_secs(60 * 60)));
}
