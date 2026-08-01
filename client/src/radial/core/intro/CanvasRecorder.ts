/**
 * Capture a canvas to a file.
 *
 * How the marketing assets get made. Alpha survives into a VP9 WebM when the
 * sequence is configured with `background: null`, which is what an editor timeline
 * that wants a transparent overlay needs:
 *
 *   ffmpeg -c:v libvpx-vp9 -i intro.webm \
 *     -c:v prores_ks -profile:v 4444 -pix_fmt yuva444p10le intro.mov
 */
export class CanvasRecorder {
  static readonly MIME_PREFERENCE = ["video/webm;codecs=vp9", "video/webm;codecs=vp8", "video/webm"];

  static isSupported(): boolean {
    return typeof MediaRecorder !== "undefined" && typeof HTMLCanvasElement !== "undefined";
  }

  static toWebM(canvas: HTMLCanvasElement, seconds: number, fps = 60): Promise<Blob> {
    const stream = canvas.captureStream(fps);
    const mimeType = CanvasRecorder.MIME_PREFERENCE.find((m) => MediaRecorder.isTypeSupported(m)) ?? "";
    const recorder = new MediaRecorder(stream, { mimeType, videoBitsPerSecond: 12_000_000 });
    const chunks: Blob[] = [];

    return new Promise((resolve, reject) => {
      recorder.ondataavailable = (e) => {
        if (e.data.size) chunks.push(e.data);
      };
      recorder.onerror = () => reject(new Error("radial: recording failed"));
      recorder.onstop = () => resolve(new Blob(chunks, { type: mimeType || "video/webm" }));
      recorder.start();
      setTimeout(() => recorder.stop(), seconds * 1000);
    });
  }

  static toPng(canvas: HTMLCanvasElement): Promise<Blob> {
    return new Promise((resolve, reject) => {
      canvas.toBlob((blob) => {
        if (blob) resolve(blob);
        else reject(new Error("radial: frame capture failed"));
      }, "image/png");
    });
  }

  static download(blob: Blob, filename: string): void {
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = filename;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 4000);
  }
}
