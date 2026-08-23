import { I18n } from '$lib/i18n';

/** Remembered `world_uuid` → the name the reader chose for it in BVC Connect. */
export type WorldAssociations = Readonly<Record<string, string>>;

/**
 * What to call a world in the chat surfaces.
 *
 * `world_name` arrives from whatever is relaying that world, and for most worlds it is not a
 * name at all: BDS cannot read the level name so the mod sends the world's uuid, Paper and
 * Fabric report a level name whose default is literally `world`, and the proxy path derives a
 * hash. A list reading "Minecraft world", "world", "world" names nothing.
 *
 * So a placeholder is detected by shape and replaced with a name the reader has actually seen —
 * the entry they picked in BVC Connect, remembered against the uuid the first time they
 * connected through it.
 */
export class WorldLabel {
    /** Level names that identify nothing because nearly every world carries them. */
    private static readonly DEFAULT_NAMES: readonly string[] = [
        'world',
        'minecraft world',
        'bedrock level',
        'dedicated server',
        'my world',
    ];

    private static readonly UUID = /^[0-9a-f]{8}-?[0-9a-f]{4}-?[0-9a-f]{4}-?[0-9a-f]{4}-?[0-9a-f]{12}$/i;

    /** The proxy path's `BedrockWorldId::derive` output: a bare hex digest, never a name. */
    private static readonly HEX_ID = /^[0-9a-f]{12,}$/i;

    /**
     * Whether this is an identifier or a shared default rather than something a person chose.
     */
    static isPlaceholder(worldName: string): boolean {
        const trimmed = worldName.trim();
        if (trimmed.length === 0) {
            return true;
        }
        if (WorldLabel.UUID.test(trimmed) || WorldLabel.HEX_ID.test(trimmed)) {
            return true;
        }
        return WorldLabel.DEFAULT_NAMES.includes(trimmed.toLowerCase());
    }

    /**
     * The label to render for a world.
     *
     * A name the operator set always wins — they chose it, and a remembered association is only
     * ever this client's guess. Otherwise the remembered name, and failing that an honest
     * "unnamed" carrying a short id so two unnamed worlds are still distinguishable.
     */
    static resolve(
        worldUuid: string,
        worldName: string,
        associations: WorldAssociations,
    ): string {
        if (!WorldLabel.isPlaceholder(worldName)) {
            return worldName.trim();
        }

        const remembered = associations[worldUuid]?.trim();
        if (remembered && remembered.length > 0) {
            return remembered;
        }

        return I18n.tf('Unnamed world ({id})', { id: WorldLabel.shortId(worldUuid) });
    }

    /** Enough of the id to tell two unnamed worlds apart without rendering the whole thing. */
    private static shortId(worldUuid: string): string {
        const bare = worldUuid.replace(/-/g, '');
        return bare.slice(0, 8) || worldUuid;
    }
}
