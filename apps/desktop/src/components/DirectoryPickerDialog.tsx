import { useEffect, useRef, useState } from "react";
import type { CraftelApi } from "../api/craftel";
import { errorMessage } from "../api/craftel";
import type { DirectoryEntry, DirectoryListing } from "../api/types";

export function DirectoryPickerDialog({ api, opener, onCancel, onAdd }: { api: CraftelApi; opener: React.RefObject<HTMLElement | null>; onCancel(): void; onAdd(path: string): Promise<boolean> }) {
  const [listing, setListing] = useState<DirectoryListing | null>(null);
  const [path, setPath] = useState("");
  const [selected, setSelected] = useState<DirectoryEntry | null>(null);
  const [loading, setLoading] = useState(true);
  const [adding, setAdding] = useState(false);
  const [error, setError] = useState("");
  const dialog = useRef<HTMLElement>(null);
  const request = useRef(0);
  const pending = useRef(false);

  const navigate = async (next?: string) => {
    const currentRequest = ++request.current;
    setLoading(true);
    setError("");
    try {
      const value = await api.listDirectory(next);
      if (request.current !== currentRequest) return;
      setListing(value);
      setPath(value.path);
      setSelected(null);
    } catch (cause) {
      if (request.current === currentRequest) setError(errorMessage(cause));
    } finally {
      if (request.current === currentRequest) setLoading(false);
    }
  };

  useEffect(() => {
    void navigate();
    const backgrounds = [...document.querySelectorAll<HTMLElement>(".shell > main, .projects > :not(.scrim)")];
    const keydown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !pending.current) onCancel();
      if (event.key !== "Tab" || !dialog.current) return;
      const focusable = [...dialog.current.querySelectorAll<HTMLElement>('button:not([disabled]), input:not([disabled]), [tabindex="0"]')];
      if (!focusable.length) return;
      const first = focusable[0], last = focusable.at(-1)!;
      if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last.focus(); }
      else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus(); }
    };
    document.addEventListener("keydown", keydown);
    backgrounds.forEach(node => node.setAttribute("inert", ""));
    return () => { request.current += 1; document.removeEventListener("keydown", keydown); backgrounds.forEach(node => node.removeAttribute("inert")); opener.current?.focus(); };
  }, []);

  const add = async () => {
    const chosen = selected?.path ?? listing?.path;
    if (!chosen) return;
    pending.current = true;
    setAdding(true);
    setError("");
    try { if (!await onAdd(chosen)) { pending.current = false; setAdding(false); } }
    catch (cause) { pending.current = false; setError(errorMessage(cause)); setAdding(false); }
  };

  const pathMatchesListing = path.trim() === listing?.path;

  return <div className="scrim">
    <section ref={dialog} className="directory-dialog" role="dialog" aria-modal="true" aria-labelledby="directory-title" aria-describedby="directory-help">
      <h2 id="directory-title">Folder path</h2>
      <form className="directory-path" onSubmit={event => { event.preventDefault(); void navigate(path.trim()); }}>
        <button type="button" aria-label="Go to parent folder" disabled={loading || !listing?.parent} onClick={() => void navigate(listing?.parent ?? undefined)}>←</button>
        <label className="sr-only" htmlFor="directory-path-input">Current folder path</label>
        <input id="directory-path-input" autoFocus value={path} onChange={event => { setPath(event.target.value); setSelected(null); }} aria-invalid={!!error} spellCheck={false} />
        <button className="sr-only" type="submit">Go to path</button>
      </form>
      <div className="directory-list" role="group" aria-label="Subfolders" aria-busy={loading} aria-describedby="directory-keyboard-help">
        {loading ? <p role="status">Loading folders…</p> : listing?.entries.length ? listing.entries.map(entry =>
          <button key={entry.path} type="button" aria-pressed={selected?.path === entry.path} onClick={() => setSelected(entry)} onDoubleClick={() => void navigate(entry.path)} onKeyDown={event => { if (event.key === "ArrowRight") { event.preventDefault(); void navigate(entry.path); } }}>
            <span aria-hidden="true">▱</span><span>{entry.name}</span>{entry.hidden && <span className="sr-only"> hidden folder</span>}
          </button>
        ) : <p role="status">This folder has no subfolders.</p>}
      </div>
      <p id="directory-keyboard-help" className="sr-only">Select a folder with Enter. Open it with the Right Arrow key.</p>
      {error && <p role="alert">{error}</p>}
      <p id="directory-help" className="directory-help">The selected folder will appear as a separate project in the sidebar. If no subfolder is selected, the current folder is added.</p>
      <div className="actions"><button type="button" disabled={adding} onClick={onCancel}>Cancel</button><button type="button" className="primary" disabled={loading || adding || !listing || !pathMatchesListing} onClick={() => void add()}>{adding ? "Adding…" : "Add Project"}</button></div>
    </section>
  </div>;
}
