/// What a caller must say to start the proxy against a Realm.
///
/// There is no advertised-protocol field: a Realm's version is resolved from the feature
/// flag rather than chosen per connection, so offering an override here would be a lie.
pub struct RealmConnectRequest {
    pub realm_id: u64,
    /// Shown on the connection card and recorded as the active realm, so it travels with
    /// the id rather than being looked up again after the session is already running.
    pub realm_name: String,
    pub network_interface: Option<String>,
}
