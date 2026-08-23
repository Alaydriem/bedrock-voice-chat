/// The set of metrics this server exposes. Centralizes metric identity so names
/// live in one place instead of scattered string constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Metric {
    PlayerConnectionsTotal,
    PlayerDisconnectionsTotal,
    SessionDurationSeconds,
    ActivePlayers,
    PeakPlayers,
    PlayersReached,
    PlayersReachedMutual,
    ActiveChannels,
    PlayersInChannels,
    ChannelJoinsTotal,
    ChannelLeavesTotal,
    AudioFramesRoutedTotal,
    AudioRouteDurationSeconds,
    AudioRouteRecipientDropsTotal,
    PositionDatagramsTotal,
    PositionDatagramBytes,
    PositionPlayersAdvertisedTotal,
    PositionOversizeDropsTotal,
    WebsocketHandshakeRejectionsTotal,
    VoiceCapacityLimit,
    ConnectionsRefusedTotal,
    BuildInfo,
}

impl Metric {
    pub fn name(&self) -> &'static str {
        match self {
            Metric::WebsocketHandshakeRejectionsTotal => "bvc_websocket_handshake_rejections_total",
            Metric::PlayerConnectionsTotal => "bvc_player_connections_total",
            Metric::PlayerDisconnectionsTotal => "bvc_player_disconnections_total",
            Metric::SessionDurationSeconds => "bvc_session_duration_seconds",
            Metric::ActivePlayers => "bvc_active_players",
            Metric::PeakPlayers => "bvc_peak_players",
            Metric::PlayersReached => "bvc_players_reached",
            Metric::PlayersReachedMutual => "bvc_players_reached_mutual",
            Metric::ActiveChannels => "bvc_active_channels",
            Metric::PlayersInChannels => "bvc_players_in_channels",
            Metric::ChannelJoinsTotal => "bvc_channel_joins_total",
            Metric::ChannelLeavesTotal => "bvc_channel_leaves_total",
            Metric::AudioFramesRoutedTotal => "bvc_audio_frames_routed_total",
            Metric::AudioRouteDurationSeconds => "bvc_audio_route_duration_seconds",
            Metric::AudioRouteRecipientDropsTotal => "bvc_audio_route_recipient_drops_total",
            Metric::PositionDatagramsTotal => "bvc_position_datagrams_total",
            Metric::PositionDatagramBytes => "bvc_position_datagram_bytes",
            Metric::PositionPlayersAdvertisedTotal => "bvc_position_players_advertised_total",
            Metric::PositionOversizeDropsTotal => "bvc_position_oversize_drops_total",
            Metric::VoiceCapacityLimit => "bvc_voice_capacity_limit",
            Metric::ConnectionsRefusedTotal => "bvc_connections_refused_total",
            Metric::BuildInfo => "bvc_build_info",
        }
    }

    /// Counter families that must be pre-registered at 0 so an idle server's
    /// `/metrics` shows them (metrics-rs registers lazily on first emission).
    pub fn counters() -> [Metric; 10] {
        [
            Metric::PlayerConnectionsTotal,
            Metric::PlayerDisconnectionsTotal,
            Metric::ChannelJoinsTotal,
            Metric::ChannelLeavesTotal,
            Metric::AudioFramesRoutedTotal,
            Metric::AudioRouteRecipientDropsTotal,
            Metric::PositionDatagramsTotal,
            Metric::PositionPlayersAdvertisedTotal,
            Metric::PositionOversizeDropsTotal,
            Metric::ConnectionsRefusedTotal,
        ]
    }
}
