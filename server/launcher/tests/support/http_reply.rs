/// What `HttpFixture` answers for one path.
#[derive(Clone)]
pub enum HttpReply {
    Body(Vec<u8>),
    Status(u16),
    // Accepts the request and never answers.
    Stall,
}
