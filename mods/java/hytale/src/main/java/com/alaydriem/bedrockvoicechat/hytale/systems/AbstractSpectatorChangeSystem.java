package com.alaydriem.bedrockvoicechat.hytale.systems;

import com.hypixel.hytale.component.CommandBuffer;
import com.hypixel.hytale.component.ComponentType;
import com.hypixel.hytale.component.Ref;
import com.hypixel.hytale.component.Store;
import com.hypixel.hytale.component.query.Query;
import com.hypixel.hytale.component.system.RefChangeSystem;
import com.hypixel.hytale.server.core.modules.entity.component.HiddenFromAdventurePlayers;
import com.hypixel.hytale.server.core.universe.PlayerRef;
import com.hypixel.hytale.server.core.universe.world.storage.EntityStore;

/**
 * Java base class for spectator mode detection.
 * Extend this in Kotlin to implement spectator callbacks.
 */
public abstract class AbstractSpectatorChangeSystem extends RefChangeSystem<EntityStore, HiddenFromAdventurePlayers> {

    @Override
    public Query<EntityStore> getQuery() {
        return Query.any();
    }

    @Override
    public ComponentType<EntityStore, HiddenFromAdventurePlayers> componentType() {
        return HiddenFromAdventurePlayers.getComponentType();
    }

    @Override
    public void onComponentAdded(
            Ref<EntityStore> ref,
            HiddenFromAdventurePlayers component,
            Store<EntityStore> store,
            CommandBuffer<EntityStore> commandBuffer
    ) {
        PlayerRef playerRef = store.getComponent(ref, PlayerRef.getComponentType());
        if (playerRef != null) {
            onSpectatorChanged(playerRef, true);
        }
    }

    @Override
    public void onComponentSet(
            Ref<EntityStore> ref,
            HiddenFromAdventurePlayers oldComponent,
            HiddenFromAdventurePlayers newComponent,
            Store<EntityStore> store,
            CommandBuffer<EntityStore> commandBuffer
    ) {
        // Component updated - not typically relevant for spectator
    }

    @Override
    public void onComponentRemoved(
            Ref<EntityStore> ref,
            HiddenFromAdventurePlayers component,
            Store<EntityStore> store,
            CommandBuffer<EntityStore> commandBuffer
    ) {
        PlayerRef playerRef = store.getComponent(ref, PlayerRef.getComponentType());
        if (playerRef != null) {
            onSpectatorChanged(playerRef, false);
        }
    }

    /** Called when a player's spectator state changes. */
    protected abstract void onSpectatorChanged(PlayerRef playerRef, boolean spectator);
}
