import { useEffect, useRef, useState } from "react";
import type { UpdateInfo, UpdateService } from "../api/updater";

const LAST_CHECK = "craftel.update.lastCheck";
const DAY = 24 * 60 * 60 * 1000;
const errorMessage = (error: unknown) => error instanceof Error ? error.message : String(error);

export function UpdateControl({ service, now = Date.now }: { service: UpdateService | null; now?: () => number }) {
  const [update, setUpdate] = useState<UpdateInfo | null>(null), [message, setMessage] = useState(""), [busy, setBusy] = useState(false), [installed, setInstalled] = useState(false);
  const current = useRef<UpdateInfo | null>(null), mounted = useRef(true);
  const dispose = (value: UpdateInfo | null) => { if (value) void value.dispose().catch(() => {}); };
  const runCheck = async (manual: boolean) => {
    if (!service) return;
    localStorage.setItem(LAST_CHECK, String(now()));
    setBusy(true);
    setMessage(manual ? "Checking for updates…" : "");
    try {
      const found = await service.check();
      if (!mounted.current) { dispose(found); return; }
      dispose(current.current);
      current.current = found;
      setUpdate(found);
      setMessage(found ? `Version ${found.version} is available.` : manual ? "CRAFTEL is up to date." : "");
    } catch (error) {
      if (mounted.current) setMessage(`Update check failed: ${errorMessage(error)}`);
    } finally {
      if (mounted.current) setBusy(false);
    }
  };
  useEffect(() => {
    mounted.current = true;
    if (service) {
      const last = Number(localStorage.getItem(LAST_CHECK) || 0);
      if (now() - last >= DAY) void runCheck(false);
    }
    return () => { mounted.current = false; dispose(current.current); current.current = null; };
  }, [service]);
  const download = async () => {
    if (!update) return;
    setBusy(true);
    try {
      await update.downloadAndInstall((bytes, total) => setMessage(total ? `Downloading ${bytes.toLocaleString()} / ${total.toLocaleString()} bytes…` : `Downloading ${bytes.toLocaleString()} bytes…`));
      dispose(update);
      current.current = null;
      setUpdate(null);
      setInstalled(true);
      setMessage("Update installed. Restart to finish.");
    } catch (error) {
      setMessage(`Update failed: ${errorMessage(error)}`);
    } finally {
      setBusy(false);
    }
  };
  const restart = async () => {
    if (!service) return;
    setBusy(true);
    try { await service.relaunch(); }
    catch (error) { setMessage(`Restart failed: ${errorMessage(error)}`); }
    finally { setBusy(false); }
  };
  if (!service) return null;
  return <section className="update-control" aria-label="Application update"><p role={message.includes("failed") ? "alert" : "status"}>{message}{update?.notes && !installed ? ` ${update.notes}` : ""}</p>{installed ? <button disabled={busy} onClick={() => void restart()}>Restart to update</button> : update ? <button disabled={busy} onClick={() => void download()}>Download</button> : <button disabled={busy} onClick={() => void runCheck(true)}>Check for updates</button>}</section>;
}
