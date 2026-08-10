import type { Entity, EntityDamageSource, Player } from '@minecraft/server';
import { EntityDamageCause } from '@minecraft/server';

/**
 * Words a death the way the game words it.
 *
 * The scripting API reports a cause and a killer rather than the line players read, so the
 * line has to be rebuilt. The wording is taken from the table the no-net path already renders
 * (`MinecraftTranslation`), so a death reads identically whether it reached the app through a
 * mod or through the proxy.
 *
 * Curated rather than exhaustive, for the same reason that table is: a cause with no entry
 * falls back to the generic pair, which is what the proxy does with a translation key it does
 * not know.
 */
export class DeathMessage {
  private static readonly UNNAMED = 'something';

  static render(victim: string, source: EntityDamageSource | undefined): string {
    const killer = DeathMessage.killerOf(source);
    const template = DeathMessage.template(source?.cause, killer !== null);
    return template.replace('%1', victim).replace('%2', killer ?? DeathMessage.UNNAMED);
  }

  /**
   * Templates carry `%1` for the victim and `%2` for the killer.
   *
   * `named` splits the causes that word themselves differently with and without a killer.
   * Reading `%2` when nothing killed the player produces "was slain by something", which is
   * worse than the plain form.
   */
  private static template(cause: EntityDamageCause | undefined, named: boolean): string {
    switch (cause) {
      case EntityDamageCause.anvil:
        return '%1 was squashed by a falling anvil';
      case EntityDamageCause.blockExplosion:
        return '%1 blew up';
      case EntityDamageCause.campfire:
      case EntityDamageCause.soulCampfire:
      case EntityDamageCause.fire:
        return '%1 went up in flames';
      case EntityDamageCause.contact:
        return '%1 was pricked to death';
      case EntityDamageCause.drowning:
        return '%1 drowned';
      case EntityDamageCause.entityAttack:
        return named ? '%1 was slain by %2' : '%1 was slain';
      case EntityDamageCause.entityExplosion:
        return named ? '%1 was blown up by %2' : '%1 blew up';
      case EntityDamageCause.fall:
        return '%1 fell from a high place';
      case EntityDamageCause.fallingBlock:
        return '%1 was squashed by a falling block';
      case EntityDamageCause.fireTick:
        return '%1 burned to death';
      case EntityDamageCause.fireworks:
        return '%1 went off with a bang';
      case EntityDamageCause.flyIntoWall:
        return '%1 experienced kinetic energy';
      case EntityDamageCause.freezing:
      case EntityDamageCause.temperature:
        return '%1 froze to death';
      case EntityDamageCause.lava:
        return '%1 tried to swim in lava';
      case EntityDamageCause.lightning:
        return '%1 was struck by lightning';
      case EntityDamageCause.maceSmash:
        return named ? '%1 was smashed by %2' : '%1 died';
      case EntityDamageCause.magic:
        return named ? '%1 was killed by %2 using magic' : '%1 was killed by magic';
      case EntityDamageCause.magma:
        return '%1 discovered the floor was lava';
      case EntityDamageCause.projectile:
        return named ? '%1 was shot by %2' : '%1 was shot';
      case EntityDamageCause.ramAttack:
        return named ? '%1 was rammed to death by %2' : '%1 died';
      case EntityDamageCause.sonicBoom:
        return '%1 was obliterated by a sonically-charged shriek';
      case EntityDamageCause.stalactite:
        return '%1 was skewered by a falling stalactite';
      case EntityDamageCause.stalagmite:
        return '%1 was impaled on a stalagmite';
      case EntityDamageCause.starve:
        return '%1 starved to death';
      case EntityDamageCause.suffocation:
        return '%1 suffocated in a wall';
      case EntityDamageCause.thorns:
        return named ? '%1 was killed trying to hurt %2' : '%1 died';
      case EntityDamageCause.void:
        return '%1 fell out of the world';
      case EntityDamageCause.wither:
        return '%1 withered away';
      default:
        return named ? '%1 was killed by %2' : '%1 died';
    }
  }

  /**
   * Who to name, or null when nothing identifiable did it.
   *
   * The shooter is named ahead of the arrow: `damagingEntity` is the mob or player that fired,
   * and naming the projectile instead loses the only part anyone cares about. A projectile
   * with no shooter still reads better than nothing.
   */
  private static killerOf(source: EntityDamageSource | undefined): string | null {
    if (!source) {
      return null;
    }
    return (
      DeathMessage.nameOf(source.damagingEntity) ??
      DeathMessage.nameOf(source.damagingProjectile)
    );
  }

  /**
   * Every read here can throw: the killer may already have been removed by the time the death
   * is reported, and touching a removed entity raises rather than returning undefined. A death
   * line is not worth losing to that, so an unreadable killer is simply not named.
   */
  private static nameOf(entity: Entity | undefined): string | null {
    if (!entity) {
      return null;
    }

    try {
      if (entity.typeId === 'minecraft:player') {
        return (entity as Player).name;
      }
      if (entity.nameTag) {
        return entity.nameTag;
      }
      return DeathMessage.readableType(entity.typeId);
    } catch {
      return null;
    }
  }

  /** `minecraft:cave_spider` reads as `Cave Spider`. */
  private static readableType(typeId: string): string {
    const bare = typeId.includes(':') ? typeId.slice(typeId.indexOf(':') + 1) : typeId;
    return bare
      .split('_')
      .filter((word) => word.length > 0)
      .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
      .join(' ');
  }
}
