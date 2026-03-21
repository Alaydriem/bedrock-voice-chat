
import { mkdir, writeFile, exists, readFile, stat } from '@tauri-apps/plugin-fs';
import { appCacheDir } from '@tauri-apps/api/path';
import { debug } from '@tauri-apps/plugin-log';
import axios from "axios";

// @ts-ignore
import murmurHash3 from "murmurhash3js";
import type ImageCacheOptions from './imageCacheOptions';

export default class ImageCache {
    /**
     * Fetches an image from disk or from a url, caches it locally, then returns the base64 data stream.
     * @param options ImageCacheOptions
     * @returns
     */
    async getImage(options: ImageCacheOptions): Promise<string> {
        if (!options.url || !options.url.startsWith("http")) return "";

        const cacheDir = await appCacheDir() + "/images";
        const hash = this.hashImageUrl(options.url);
        const cachedImagePath = `${cacheDir}/${hash}`;

        // Ensure the cache directory exists
        if (!await exists(cacheDir)) {
            await mkdir(cacheDir, { recursive: true });
        }

        // Check if the file exists on disk
        if (await exists(cachedImagePath)) {
            const isExpired = await this.isCacheExpired(cachedImagePath, options.ttl);
            if (!isExpired) {
                const fileData = await readFile(cachedImagePath);
                const mimeType = this.getMimeType(fileData);
                const base64Data = this.arrayBufferToBase64(fileData);
                return `data:${mimeType};base64,${base64Data}`;
            }
        }

        // If the file doesn't exist or is expired, fetch it from the remote destination
        try {
            const response = await axios.get(options.url, { responseType: "arraybuffer" });
            const imageData = new Uint8Array(response.data);
            const mimeType = this.getMimeType(imageData);
            await writeFile(cachedImagePath, imageData);
            const base64Data = this.arrayBufferToBase64(imageData);
            return `data:${mimeType};base64,${base64Data}`;
        } catch (err) {
            debug(`Could not fetch image ${options.url}: ${err}`);
            return "";
        }
    }

    async isCacheExpired(path: string, ttlSeconds: number): Promise<boolean> {
        try {
            const fileStat = await stat(path);
            if (!fileStat.mtime) {
                return true;
            }
            const ageMs = Date.now() - fileStat.mtime.getTime();
            return ageMs > ttlSeconds * 1000;
        } catch {
            return true;
        }
    }

    /**
     * Hashes the image URL using MurmurHash3 to create a unique identifier for the image
     * @param url
     * @returns
     */
    hashImageUrl(url: string): string {
        const bytes = new TextEncoder().encode(url);
        const byteString = Array.from(bytes)
            .map((byte) => String.fromCharCode(byte))
            .join('');
        return murmurHash3.x86.hash128(byteString);
    }

    /**
     * Determines the MIME type of the file based upon it's header signature.
     * @param data
     * @returns
     */
    getMimeType(data: Uint8Array | string): string {
        const bytes = typeof data === "string" ? new Uint8Array(data.split("").map((char) => char.charCodeAt(0))) : data;

        // Check for PNG signature (first 8 bytes: 89 50 4E 47 0D 0A 1A 0A)
        if (bytes[0] === 0x89 && bytes[1] === 0x50 && bytes[2] === 0x4E && bytes[3] === 0x47) {
            return "image/png";
        }

        // Check for JPEG signature (first 3 bytes: FF D8 FF)
        if (bytes[0] === 0xFF && bytes[1] === 0xD8 && bytes[2] === 0xFF) {
            return "image/jpeg";
        }

        // Default to binary/octet-stream if unknown
        return "application/octet-stream";
    }

    /**
     * Returns a base64 string from a Uint8Array of the image data
     * @param blob
     * @returns
     */
    arrayBufferToBase64(blob: Uint8Array) {
        // Assuming 'uint8Array' is your Uint8Array
        return btoa(
            new Uint8Array(blob).reduce(function (data, byte) {
            return data + String.fromCharCode(byte);
            }, ""),
        );
    }
}