package com.alaydriem.bedrockvoicechat.paper

import com.google.gson.JsonArray
import com.google.gson.JsonElement
import com.google.gson.JsonNull
import com.google.gson.JsonObject
import com.google.gson.JsonPrimitive
import org.bukkit.configuration.ConfigurationSection

/**
 * Converts a Bukkit configuration tree into JSON, with no knowledge of any
 * particular key. Reading the whole document in one step is what keeps a Paper
 * config block from falling behind the shape the mod accepts.
 */
object YamlSectionConverter {

    fun toJson(section: ConfigurationSection): JsonObject {
        val json = JsonObject()
        for (key in section.getKeys(false)) {
            json.add(key, convert(section.get(key)))
        }
        return json
    }

    private fun convert(value: Any?): JsonElement = when (value) {
        null -> JsonNull.INSTANCE
        is ConfigurationSection -> toJson(value)
        is Map<*, *> -> {
            val json = JsonObject()
            for ((key, entry) in value) {
                json.add(key.toString(), convert(entry))
            }
            json
        }
        is List<*> -> {
            val array = JsonArray()
            for (entry in value) {
                array.add(convert(entry))
            }
            array
        }
        is Boolean -> JsonPrimitive(value)
        is Number -> JsonPrimitive(value)
        else -> JsonPrimitive(value.toString())
    }
}
