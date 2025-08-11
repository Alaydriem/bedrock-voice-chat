pub(crate) enum StreamTraitType {
    QuicListener(QuicListener),
    CacheManager(CacheManager),
    WebhookReceiver(WebhookReceiver)
}