import type ImageCache from '../../components/imageCache';
import type { ServerHealthService } from '../../services/ServerHealthService';
import type { ServerListStore } from '../../services/ServerListStore';

export interface ServerCardManagerDeps {
    health: ServerHealthService;
    serverList: ServerListStore;
    imageCache: ImageCache;
}
