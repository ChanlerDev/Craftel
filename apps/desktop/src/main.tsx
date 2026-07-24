import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { tauriApi } from "./api/tauri";
import type { CraftelApi } from "./api/craftel";
import "@fontsource-variable/geist";
import "@fontsource-variable/geist-mono";
import "./styles.css";

let api: CraftelApi = tauriApi;
// Vite replaces DEV with false, so production cannot import or activate mock mode.
if (import.meta.env.DEV && (new URLSearchParams(location.search).get("mock") === "1" || !("__TAURI_INTERNALS__" in window))) {
  api = (await import("./api/mock")).mockApi;
}
createRoot(document.getElementById("root")!).render(<StrictMode><App api={api} /></StrictMode>);
