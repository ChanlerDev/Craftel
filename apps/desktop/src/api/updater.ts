import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export interface UpdateInfo { version: string; notes?: string; downloadAndInstall(onProgress: (downloaded: number, total?: number) => void): Promise<void>; dispose(): Promise<void> }
export interface UpdateService { check(): Promise<UpdateInfo | null>; relaunch(): Promise<void> }

function wrap(update: Update): UpdateInfo {
  return { version: update.version, notes: update.body,
    async downloadAndInstall(progress) { let downloaded = 0; let total: number | undefined;
      await update.downloadAndInstall(event => { if (event.event === "Started") total = event.data.contentLength; if (event.event === "Progress") downloaded += event.data.chunkLength; progress(downloaded, total); });
    },
    dispose: () => update.close(),
  };
}

export const nativeUpdateService: UpdateService | null = "__TAURI_INTERNALS__" in window
  ? { async check() { const update = await check(); return update ? wrap(update) : null; }, relaunch }
  : null;
