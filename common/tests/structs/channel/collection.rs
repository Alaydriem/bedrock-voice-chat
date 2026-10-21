use common::errors::ProtectRefusal;
use common::structs::channel::{Channel, ChannelCollection};

fn identity(canonical: &str) -> common::PlayerIdentity {
    canonical.parse().expect("canonical identity")
}

async fn alone(owner: &common::PlayerIdentity) -> (ChannelCollection, String) {
    let channels = ChannelCollection::new(64);
    let channel = Channel::new("Night Raid".into(), owner.clone());
    let id = channel.id();
    channels.insert(channel).await;
    channels.add_player_to_channel(owner, &id).await;
    (channels, id)
}

#[tokio::test]
async fn protect_if_alone_protects_an_owner_alone() {
    let owner = identity("minecraft:Alice");
    let (channels, id) = alone(&owner).await;

    assert_eq!(channels.protect_if_alone(&id, &owner).await, Ok(()));
    assert!(channels.get(&id).await.unwrap().is_protected());
}

#[tokio::test]
async fn protect_if_alone_refuses_each_case() {
    let owner = identity("minecraft:Alice");
    let (channels, id) = alone(&owner).await;

    assert_eq!(
        channels.protect_if_alone(&id, &identity("minecraft:Bob")).await,
        Err(ProtectRefusal::NotCreator)
    );
    assert_eq!(
        channels.protect_if_alone("ZZZZZZZZ", &owner).await,
        Err(ProtectRefusal::NotFound)
    );
    channels.add_player_to_channel(&identity("minecraft:Bob"), &id).await;
    assert_eq!(
        channels.protect_if_alone(&id, &owner).await,
        Err(ProtectRefusal::NotAlone)
    );
    assert!(!channels.get(&id).await.unwrap().is_protected());
}

#[tokio::test]
async fn an_open_join_is_refused_by_a_protected_channel() {
    let owner = identity("minecraft:Alice");
    let (channels, id) = alone(&owner).await;
    channels.protect_if_alone(&id, &owner).await.unwrap();

    assert!(!channels.add_player_to_open_channel(&identity("minecraft:Bob"), &id).await);
    assert!(!channels.get(&id).await.unwrap().contains(&identity("minecraft:Bob")));
}

// A join and a protect that land at the same moment must leave one of two states: protected
// with the owner alone, or open with both. Never protected with a player who holds no keys,
// and never open again after the owner's app started the group's keys.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_join_racing_a_protect_never_leaves_a_mixed_channel() {
    let owner = identity("minecraft:Alice");
    let bob = identity("minecraft:Bob");
    for _ in 0..300 {
        let (channels, id) = alone(&owner).await;
        let (protect, join) = tokio::join!(
            {
                let channels = channels.clone();
                let id = id.clone();
                let owner = owner.clone();
                tokio::spawn(async move { channels.protect_if_alone(&id, &owner).await })
            },
            {
                let channels = channels.clone();
                let id = id.clone();
                let bob = bob.clone();
                tokio::spawn(async move { channels.add_player_to_open_channel(&bob, &id).await })
            },
        );
        let protected = protect.unwrap().is_ok();
        let joined = join.unwrap();
        let channel = channels.get(&id).await.unwrap();

        assert_ne!(protected, joined, "exactly one of the two may win");
        assert_eq!(channel.is_protected(), protected);
        assert_eq!(channel.contains(&bob), joined);
    }
}
