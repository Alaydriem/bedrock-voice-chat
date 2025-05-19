export default class ImageCacheOptions {
    url: string;
    ttl: number;
    
    constructor(url: string, ttl: number) {
        this.url = url;
        this.ttl = ttl;
    }
}