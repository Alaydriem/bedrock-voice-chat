use common::net::srv::{SrvAnswer, SrvRecordChoice};

const HOST: &str = "brave.example.com";

struct Answers;

impl Answers {
    fn to(target: &str, priority: u16, weight: u16, port: u16) -> SrvAnswer {
        SrvAnswer {
            priority,
            weight,
            port,
            target: target.to_string(),
            ttl: 60,
        }
    }
}

#[test]
fn a_dot_target_means_no_service() {
    assert!(SrvRecordChoice::pick(HOST, &[Answers::to(".", 0, 0, 1001)]).is_none());
}

#[test]
fn a_different_target_is_ignored() {
    assert!(SrvRecordChoice::pick(HOST, &[Answers::to("other.example.com", 0, 0, 1001)]).is_none());
}

#[test]
fn a_port_of_zero_is_ignored() {
    assert!(SrvRecordChoice::pick(HOST, &[Answers::to(HOST, 0, 0, 0)]).is_none());
}

#[test]
fn lowest_priority_then_highest_weight_then_lowest_port_wins() {
    let answers = [
        Answers::to(HOST, 10, 0, 1001),
        Answers::to(HOST, 0, 5, 1002),
        Answers::to(HOST, 0, 9, 1004),
        Answers::to(HOST, 0, 9, 1003),
    ];
    assert_eq!(SrvRecordChoice::pick(HOST, &answers).unwrap().port, 1003);
}

#[test]
fn the_target_match_ignores_case_and_the_trailing_dot() {
    let answers = [Answers::to("Brave.Example.COM.", 0, 0, 1001)];
    assert_eq!(SrvRecordChoice::pick(HOST, &answers).unwrap().port, 1001);
}
