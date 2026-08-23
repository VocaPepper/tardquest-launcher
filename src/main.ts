import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

interface Build {
  edition: string;
  version: string;
  label: string;
  file_name: string;
  download_url: string;
  sha256?: string;
  size?: number;
  release_notes: string;
  patch?: string;
}

interface EditionInfo {
  key: string;
  title: string;
  subtitle: string;
  source: string;
}

interface ScanResult {
  install_dir: string;
  installed_versions: string[];
  installed_patch?: string;
}

const $ = (id: string): HTMLElement => document.getElementById(id) as HTMLElement;
const log = (msg: string) => {
  const t = $("log") as HTMLTextAreaElement;
  t.value += msg + "\n";
  t.scrollTop = t.scrollHeight;
};
const setProgress = (fraction: number) => {
  ($("progress") as HTMLElement).style.width = `${Math.max(0, Math.min(1, fraction)) * 100}%`;
};

let editions: EditionInfo[] = [];
let currentEdition = "";
let builds: Build[] = [];
let selectedVersion = "";
let scan: ScanResult | null = null;
let isTqo = false;
let running = false;

function editionInfo(edition: string): EditionInfo | undefined {
  return editions.find((e) => e.key === edition);
}
function selectedBuild(): Build | undefined {
  return builds.find((b) => b.version === selectedVersion);
}
function installedSet(): Set<string> {
  return new Set(scan?.installed_versions ?? []);
}

async function populateEditions() {
  editions = await invoke<EditionInfo[]>("list_catalog");
  const sel = $("edition") as HTMLSelectElement;
  sel.innerHTML = "";
  for (const e of editions) {
    const opt = document.createElement("option");
    opt.value = e.key;
    opt.textContent = e.key;
    sel.appendChild(opt);
  }
  if (editions.length) defaultSelectEdition(editions[0].key);
}

function defaultSelectEdition(edition: string) {
  currentEdition = edition;
  ($("edition") as HTMLSelectElement).value = edition;
  const info = editionInfo(edition);
  if (info) {
    ($("title") as HTMLElement).textContent = info.title;
    ($("subtitle") as HTMLElement).textContent = info.subtitle;
  }
  isTqo = info?.source === "tqo";
  refreshEdition();
}

async function refreshEdition() {
  builds = await invoke<Build[]>("fetch_channel", { edition: currentEdition });
  await fillBuilds();
  if (builds.length) {
    selectedVersion = builds[0].version;
    ($("build") as HTMLSelectElement).value = selectedVersion;
  } else {
    selectedVersion = "";
  }
  updateState();
}

async function fillBuilds() {
  const sel = $("build") as HTMLSelectElement;
  const inst = installedSet();
  sel.innerHTML = "";
  sel.disabled = builds.length === 0;
  for (const b of builds) {
    const opt = document.createElement("option");
    opt.value = b.version;
    opt.textContent = inst.has(b.version) ? `${b.label} (Installed)` : b.label;
    sel.appendChild(opt);
  }
  if (builds.some((b) => b.version === selectedVersion)) {
    sel.value = selectedVersion;
  }
}

async function doScan() {
  scan = await invoke<ScanResult>("scan_install", { edition: currentEdition });
  ($("install_path") as HTMLInputElement).value = scan.install_dir;
}

function updateAvailable(): boolean {
  const build = selectedBuild();
  const inst = installedSet();
  if (isTqo) {
    if (!build) return false;
    return !inst.has("PTE") || (build.patch !== undefined && build.patch !== scan?.installed_patch);
  }
  const latest = builds.length ? builds[0] : undefined;
  return latest ? !inst.has(latest.version) : false;
}

function updateState() {
  const build = selectedBuild();
  const inst = installedSet();
  const selectedInstalled = inst.has(selectedVersion);
  const updateAvail = updateAvailable();

  const action = $("action") as HTMLButtonElement;
  const play = $("play") as HTMLButtonElement;

  if (selectedInstalled) {
    action.textContent = "Uninstall";
    action.disabled = false;
  } else {
    action.textContent = "Download";
    action.disabled = build == null;
  }

  if (isTqo && updateAvail) {
    play.textContent = "\u25B6 UPDATE";
    play.disabled = running;
  } else {
    play.textContent = "\u25B6 PLAY";
    play.disabled = !selectedInstalled || running;
  }

  const notes = $("notes") as HTMLTextAreaElement;
  notes.value = build?.release_notes || "No release notes available for the selected version.";
}

async function doCheck() {
  setProgress(0);
  try {
    builds = await invoke<Build[]>("fetch_channel", { edition: currentEdition });
    if (builds.length) {
      selectedVersion = builds[0].version;
    }
    await doScan();
    await fillBuilds();
    updateState();
    log(updateAvailable() ? "Update available" : "Up to date");
  } catch (e) {
    log(`Failed to fetch catalog: ${e}`);
  }
}

async function onAction() {
  const build = selectedBuild();
  const selectedInstalled = installedSet().has(selectedVersion);
  if (selectedInstalled) {
    await invoke("uninstall", { edition: currentEdition, version: selectedVersion });
  } else if (build) {
    await invoke("download_and_apply", { edition: currentEdition, build });
  }
  await doScan();
  await refreshEdition();
}

async function onPlay() {
  if (isTqo && updateAvailable()) {
    const build = selectedBuild();
    if (!build) return;
    await invoke("download_and_apply", { edition: currentEdition, build });
    await doScan();
    await refreshEdition();
    return;
  }
  if (!selectedVersion) return;
  await invoke("launch_game", { edition: currentEdition, version: selectedVersion });
}

async function checkLauncherUpdate() {
  try {
    const update = await check();
    if (update) {
      await update.downloadAndInstall();
      await relaunch();
    }
  } catch {
    // silent: offline, or no update endpoint ready yet
  }
}

async function init() {
  await listen<string>("log", (e) => log(e.payload));
  await listen<number>("download-progress", (e) => setProgress(e.payload));
  await listen<boolean>("game-running", (e) => {
    running = e.payload;
    ($("running") as HTMLElement).textContent = running ? "GAME RUNNING" : "";
    updateState();
  });

  ($("privacy") as HTMLElement).addEventListener("click", () => invoke("open_privacy"));
  ($("browse") as HTMLElement).addEventListener("click", async () => {
    const dir = await open({ directory: true });
    if (typeof dir === "string" && dir) {
      await invoke("set_install_dir", { dir });
      await doScan();
      await refreshEdition();
    }
  });
  ($("check") as HTMLElement).addEventListener("click", doCheck);
  ($("action") as HTMLElement).addEventListener("click", onAction);
  ($("play") as HTMLElement).addEventListener("click", onPlay);
  ($("edition") as HTMLElement).addEventListener("change", (e) =>
    defaultSelectEdition((e.target as HTMLSelectElement).value),
  );
  ($("build") as HTMLElement).addEventListener("change", (e) => {
    selectedVersion = (e.target as HTMLSelectElement).value;
    updateState();
  });
  ($("tab-log") as HTMLElement).addEventListener("click", () => switchTab("log"));
  ($("tab-notes") as HTMLElement).addEventListener("click", () => switchTab("notes"));

  log("Fetching manifest...");
  void checkLauncherUpdate();
  await populateEditions();
  await doScan();
  await refreshEdition();
}

function switchTab(which: "log" | "notes") {
  ($("tab-log") as HTMLElement).classList.toggle("active", which === "log");
  ($("tab-notes") as HTMLElement).classList.toggle("active", which === "notes");
  ($("panel-log") as HTMLElement).classList.toggle("hidden", which !== "log");
  ($("panel-notes") as HTMLElement).classList.toggle("hidden", which !== "notes");
}

window.addEventListener("DOMContentLoaded", () => {
  init().catch((e) => {
    log(`Init failed: ${e}`);
    console.error(e);
  });
});
