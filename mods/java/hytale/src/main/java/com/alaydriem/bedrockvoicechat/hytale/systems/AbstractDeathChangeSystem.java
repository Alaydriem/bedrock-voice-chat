package com.alaydriem.bedrockvoicechat.hytale.systems;

import com.hypixel.hytale.component.CommandBuffer;
import com.hypixel.hytale.component.ComponentType;
import com.hypixel.hytale.component.Ref;
import com.hypixel.hytale.component.Store;
import com.hypixel.hytale.component.query.Query;
import com.hypixel.hytale.component.system.RefChangeSystem;
import com.hypixel.hytale.server.core.modules.entity.damage.DeathComponent;
import com.hypixel.hytale.server.core.universe.PlayerRef;
import com.hypixel.hytale.server.core.universe.world.storage.EntityStore;

/**
 * Java base class for death detection that handles the generic type erasure issues.
 * Extend this in Kotlin to implement death/respawn callbacks.
 */
public abstract class AbstractDeathChangeSystem extends RefChangeSystem<EntityStore, DeathComponent> {

    @Override
    public Query<EntityStore> getQuery() {
        return Query.any();
    }

    @Override
    public ComponentType<EntityStore, DeathComponent> componentType() {
        return DeathComponent.getComponentType();
    }

    @Override
    public void onComponentAdded(
            Ref<EntityStore> ref,
            DeathComponent component,
            Store<EntityStore> store,
            CommandBuffer<EntityStore> commandBuffer
    ) {
        PlayerRef playerRef = store.getComponent(ref, PlayerRef.getComponentType());
        if (playerRef != null) {
            onPlayerDied(playerRef);
        }
    }

    @Override
    public void onComponentSet(
            Ref<EntityStore> ref,
            DeathComponent oldComponent,
            DeathComponent newComponent,
            Store<EntityStore> store,
            CommandBuffer<EntityStore> commandBuffer
    ) {
        // Not used for death detection
    }

    @Override
    public void onComponentRemoved(
            Ref<EntityStore> ref,
            DeathComponent component,
            Store<EntityStore> store,
            CommandBuffer<EntityStore> commandBuffer
    ) {
        PlayerRef playerRef = store.getComponent(ref, PlayerRef.getComponentType());
        if (playerRef != null) {
            onPlayerRespawned(playerRef);
        }
    }

    /** Called when a player dies (DeathComponent added). */
    protected abstract void onPlayerDied(PlayerRef playerRef);

    /** Called when a player respawns (DeathComponent removed). */
    protected abstract void onPlayerRespawned(PlayerRef playerRef);
}
