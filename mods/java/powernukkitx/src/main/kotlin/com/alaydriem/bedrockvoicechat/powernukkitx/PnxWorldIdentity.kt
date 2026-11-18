package com.alaydriem.bedrockvoicechat.powernukkitx

import com.google.gson.Gson
import com.google.gson.GsonBuilder
import com.google.gson.JsonParseException
import com.google.gson.JsonParser
import org.slf4j.LoggerFactory
import java.io.File
import java.io.IOException
import java.nio.file.AtomicMoveNotSupportedException
import java.nio.file.Files
import java.nio.file.StandardCopyOption
import java.util.UUID

/**
 * The world UUID of each PowerNukkitX level.
 *
 * PowerNukkitX levels carry no UUID of their own, so one is minted per level and
 * persisted in `worlds.json`. The key is the level's folder name: the display name
 * can change without the level changing, and the folder name cannot.
 *
 * A `worlds.json` that exists but cannot be read is never written for the rest of the
 * process. The UUIDs it holds are still valid on disk, and the next start reads them.
 */
class PnxWorldIdentity(
    private val dataFolder: File,
    private val clock: () -> Long = System::currentTimeMillis,
    private val readStore: (File) -> String = { it.readText() },
) {

    private var known: MutableMap<String, String>? = null

    // Set while the cached map holds a UUID that is not yet on disk.
    private var dirty = false

    // False when the store on disk could not be read; nothing is written over it.
    private var persist = true

    private var nextSaveAt = 0L

    private var saveFailing = false

    @Synchronized
    fun uuidFor(levelFolderName: String): String {
        val worlds = known ?: load().also { known = it }
        val existing = worlds[levelFolderName]
        if (existing != null) {
            if (dirty) {
                trySave(worlds)
            }
            return existing
        }

        val minted = UUID.randomUUID().toString()
        worlds[levelFolderName] = minted
        dirty = true
        trySave(worlds)
        logger.info("Minted a world UUID for level {}", levelFolderName)
        return minted
    }

    private fun load(): MutableMap<String, String> {
        val file = File(dataFolder, FILE_NAME)
        if (!file.isFile) {
            return linkedMapOf()
        }

        val text = readWithRetries(file)
        if (text == null) {
            persist = false
            logger.error(
                "{} exists but could not be read; levels get temporary UUIDs and the file is left unchanged until restart",
                file.absolutePath,
            )
            return linkedMapOf()
        }

        val json = try {
            JsonParser.parseString(text)
        } catch (e: JsonParseException) {
            null
        }

        if (json == null || !json.isJsonObject) {
            // A level whose UUID is lost gets a new one, which splits its players'
            // world membership once. Refusing to start would cost far more.
            backUpCorrupt(file)
            return linkedMapOf()
        }

        return json.asJsonObject.entrySet()
            .filter { (_, value) -> value.isJsonPrimitive }
            .associateTo(linkedMapOf()) { (key, value) -> key to value.asString }
    }

    private fun readWithRetries(file: File): String? {
        repeat(READ_ATTEMPTS) { attempt ->
            try {
                return readStore(file)
            } catch (e: IOException) {
                if (attempt == READ_ATTEMPTS - 1) {
                    logger.error("{} could not be read", file.absolutePath, e)
                } else {
                    Thread.sleep(READ_RETRY_DELAY_MS)
                }
            }
        }
        return null
    }

    private fun backUpCorrupt(file: File) {
        val backup = File(dataFolder, "$FILE_NAME.bad")
        try {
            Files.copy(file.toPath(), backup.toPath(), StandardCopyOption.REPLACE_EXISTING)
            logger.warn("{} is corrupt and will be replaced; the old file is kept as {}", file.absolutePath, backup.name)
        } catch (e: IOException) {
            logger.warn("{} is corrupt and will be replaced; no backup could be made", file.absolutePath)
        }
    }

    // A failed save is retried no sooner than SAVE_RETRY_MS later, so a failing disk is not
    // written to, and logged about, on every lookup.
    private fun trySave(worlds: Map<String, String>) {
        if (!persist) {
            return
        }
        val now = clock()
        if (now < nextSaveAt) {
            return
        }
        if (save(worlds)) {
            dirty = false
            if (saveFailing) {
                logger.info("{} is writable again", FILE_NAME)
            }
            saveFailing = false
        } else {
            nextSaveAt = now + SAVE_RETRY_MS
        }
    }

    // True when the map reached disk.
    private fun save(worlds: Map<String, String>): Boolean {
        val target = File(dataFolder, FILE_NAME).toPath()
        return try {
            dataFolder.mkdirs()
            val staging = File(dataFolder, "$FILE_NAME.tmp").toPath()
            Files.writeString(staging, GSON.toJson(worlds))
            try {
                Files.move(staging, target, StandardCopyOption.REPLACE_EXISTING, StandardCopyOption.ATOMIC_MOVE)
            } catch (e: AtomicMoveNotSupportedException) {
                Files.move(staging, target, StandardCopyOption.REPLACE_EXISTING)
            }
            true
        } catch (e: IOException) {
            if (saveFailing) {
                logger.warn("{} still could not be written: {}", target, e.toString())
            } else {
                logger.error("{} could not be written; retrying every {} s", target, SAVE_RETRY_MS / 1000, e)
            }
            saveFailing = true
            false
        }
    }

    companion object {
        private const val FILE_NAME = "worlds.json"

        private const val READ_ATTEMPTS = 3

        private const val READ_RETRY_DELAY_MS = 100L

        private const val SAVE_RETRY_MS = 30_000L

        private val GSON: Gson = GsonBuilder().setPrettyPrinting().create()

        private val logger = LoggerFactory.getLogger("BedrockVoiceChat.Worlds")
    }
}
