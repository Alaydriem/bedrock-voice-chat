/// Whether the server still accepts the certificate this device holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CredentialVerdict {
    /// The server answered and accepted them.
    Valid,
    /// The server answered and refused them, or refused the certificate outright.
    Rejected,
    /// Nothing was established either way.
    ///
    /// A distinct answer rather than a pessimistic `Rejected`, because the caller destroys
    /// credentials on `Rejected` and an unreachable server must never be able to do that.
    Inconclusive,
}
