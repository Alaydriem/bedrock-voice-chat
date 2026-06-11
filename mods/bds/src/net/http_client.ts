import type { HttpMethod } from './http_method';
import type { HttpResponse } from './http_response';

type ServerNetModule = typeof import('@minecraft/server-net');

export class HttpClient {
  private loadPromise: Promise<void> | null = null;
  private loaded = false;
  private module: ServerNetModule | null = null;
  private warned = false;

  isAvailable(): boolean {
    return this.loaded && this.module !== null;
  }

  async ensureLoaded(): Promise<boolean> {
    if (this.loaded) {
      return this.module !== null;
    }
    if (!this.loadPromise) {
      this.loadPromise = import('@minecraft/server-net')
        .then((m) => {
          this.module = m as unknown as ServerNetModule;
        })
        .catch((e) => {
          this.module = null;
          if (!this.warned) {
            console.warn(
              '[BVC] @minecraft/server-net not available; HTTP features disabled: ' +
                e,
            );
            this.warned = true;
          }
        })
        .then(() => {
          this.loaded = true;
        });
    }
    await this.loadPromise;
    return this.module !== null;
  }

  async request(
    url: string,
    method: HttpMethod,
    body: string | undefined,
    headers: Array<[string, string]>,
    timeoutSec: number,
  ): Promise<HttpResponse | null> {
    const ok = await this.ensureLoaded();
    if (!ok || !this.module) {
      return null;
    }

    try {
      const { HttpRequest, HttpHeader, HttpRequestMethod, http } = this.module;
      const req = new HttpRequest(url);
      if (body !== undefined) {
        req.setBody(body);
      }
      const methodEnum = (
        HttpRequestMethod as unknown as Record<string, unknown>
      )[method];
      req.setMethod(methodEnum as never);
      req.setHeaders(headers.map(([n, v]) => new HttpHeader(n, v)));
      req.setTimeout(timeoutSec);

      const res = await http.request(req);
      return { status: res.status, body: res.body };
    } catch (e) {
      console.error('[BVC] HTTP request failed:', e);
      return null;
    }
  }
}

export const httpClient = new HttpClient();
