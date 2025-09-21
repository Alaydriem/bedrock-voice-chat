
import { mkdir, writeFile, exists, readFile } from '@tauri-apps/plugin-fs';
import { appCacheDir } from '@tauri-apps/api/path';
import { error, debug } from '@tauri-apps/plugin-log';
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
        const cacheDir = await appCacheDir() + "/images";
        const hash = this.hashImageUrl(options.url);
        const cachedImagePath = `${cacheDir}/${hash}`;

        // Ensure the cache directory exists
        if (!await exists(cacheDir)) {
            await mkdir(cacheDir, { recursive: true });
        }

        // Check if the file exists on disk
        if (await exists(cachedImagePath)) {
            debug(`Image found in cache: ${cachedImagePath}`);
            // Read the file and determine the MIME type
            const fileData = await readFile(cachedImagePath);
            const mimeType = this.getMimeType(fileData);
            const base64Data = this.arrayBufferToBase64(fileData);
            return `data:${mimeType};base64,${base64Data}`;
        }

        // If the file doesn't exist, fetch it from the remote destination
        return await axios.get(options.url, { responseType: "arraybuffer" }).then(async (response) => {
            if (response.status !== 200) {
                error(`Error fetching image: ${response.statusText}`);
                throw new Error(`Error fetching image: ${response.statusText}`);
            }
            return new Uint8Array(response.data);
        }).then(async (imageData) => {
            // Determine the MIME type
            const mimeType = this.getMimeType(imageData);

            // Write the image to disk
            await writeFile(cachedImagePath, imageData);
            debug(`Image cached: ${cachedImagePath}`);

            // Convert the image to a data:// scheme
            const base64Data = this.arrayBufferToBase64(imageData);
            return `data:${mimeType};base64,${base64Data}`;
        }).catch((err) => {
            error(`Error fetching or caching image: ${err}`);
            throw err;
        });
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