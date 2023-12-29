import { fs } from "@tauri-apps/api";
import { BaseDirectory } from "@tauri-apps/api/fs";
import axios from "axios";

const CACHE_DIR = "cache";

/**
 * Cache an image from a given URL to the specified cache directory.
 *
 * @param {string} imageUrl - The URL of the image to be cached.
 * @returns {Promise<void>} A promise that resolves when the image is successfully cached.
 * @throws {Error} If there's an error fetching, creating the cache directory, or writing the image data.
 */
const cacheImage = async (imageUrl: string, alias: string) => {
  try {
    const imageData = await fetchImageData(imageUrl);
    const imageName = getImageName(imageUrl, alias);
    const imagePath = getImagePath(imageName);
    await createCacheDirectory();
    await writeImageDataToCache(imagePath, imageData);
  } catch (error) {
    console.error("Error caching image:", error);
  }
};

/**
 * Fetch image data from the provided URL.
 *
 * @param {string} imageUrl - The URL of the image to fetch.
 * @returns {Promise<ArrayBuffer>} A promise that resolves to the fetched image data.
 */
const fetchImageData = async (imageUrl: string) => {
  try {
    const response = await axios.get(imageUrl, { responseType: "arraybuffer" });
    return response.data;
  } catch (error) {
    throw new Error("Error fetching image data: " + error);
  }
};

/**
 * Extracts the image name from the provided URL.
 *
 * @param {string} imageUrl - The URL of the image.
 * @returns {string} The extracted image name.
 */
const getImageName = (imageUrl: string, alias: string) => {
  if (alias == "") {
    return imageUrl.substring(imageUrl.lastIndexOf("/") + 1);
  } else {
    return alias;
  }
};

/**
 * Generates the full path to the cache directory.
 *
 * @param {string} imageName - The name of the image.
 * @returns {string} The full path to the cache directory.
 */
const getImagePath = (imageName: string) => {
  return `${CACHE_DIR}/${imageName}`;
};

/**
 * Creates the cache directory if it doesn't exist.
 *
 * @returns {Promise<void>} A promise that resolves when the cache directory is created.
 */
const createCacheDirectory = async () => {
  try {
    await fs.createDir(CACHE_DIR, {
      recursive: true,
      dir: BaseDirectory.AppData,
    });
  } catch (error) {
    throw new Error("Error creating cache directory: " + error);
  }
};

/**
 * Writes image data to the cache directory.
 *
 * @param {string} imagePath - The path to the image file.
 * @param {ArrayBuffer} imageData - The image data to write.
 * @returns {Promise<void>} A promise that resolves when the image data is written to the cache.
 */
const writeImageDataToCache = async (
  imagePath: string,
  imageData: ArrayBuffer,
) => {
  try {
    await fs.writeBinaryFile(imagePath, new Uint8Array(imageData), {
      dir: BaseDirectory.AppData,
    });
  } catch (error) {
    throw new Error("Error writing image data to cache: " + error);
  }
};

/**
 * Display a cached image or cache and display a new image.
 *
 * @param {string} imageUrl - The URL of the image to be displayed or cached.
 * @returns {Promise<string>} A promise that resolves to a base64-encoded image data URI or the original image URL.
 * @throws {Error} If there's an error reading or caching the image.
 * @example
 * const imageUrl = "https://example.com/image.jpg";
 * const cachedImage = await displayCachedImage(imageUrl);
 * console.log(cachedImage); // Outputs a base64-encoded image data URI or the original image URL.
 */
export const displayCachedImage = async (imageUrl: string, alias: string) => {
  const imageName = getImageName(imageUrl, alias);

  const imagePath = getImagePath(imageName);

  const imageExists = await fs.exists(imagePath, {
    dir: BaseDirectory.AppData,
  });

  if (imageExists) {
    // Read the binary file
    const u8Array = await fs.readBinaryFile(imagePath, {
      dir: BaseDirectory.AppData,
    });

    // Convert to base64 to consume it in the image tag
    const base64Image = _arrayBufferToBase64(u8Array);

    return base64Image;
  } else {
    // Cache the image
    cacheImage(imageUrl, alias);
    return imageUrl;
  }
};

/**
 * Converts a Uint8Array to a base64-encoded Data URI.
 *
 * @param {Uint8Array} uint8Array - The Uint8Array to convert to base64.
 * @returns {string} A Data URI in the format "data:image/jpg;base64,<base64String>".
 * @example
 * const byteArray = new Uint8Array([255, 216, 255, 224, 0, 16, 74, 70, ...]);
 * const dataUri = _arrayBufferToBase64(byteArray);
 * console.log(dataUri); // Outputs a base64-encoded Data URI.
 */
function _arrayBufferToBase64(blob: Uint8Array) {
  // Assuming 'uint8Array' is your Uint8Array
  return btoa(
    new Uint8Array(blob).reduce(function (data, byte) {
      return data + String.fromCharCode(byte);
    }, ""),
  );
}
