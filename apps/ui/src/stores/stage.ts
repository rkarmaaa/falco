// Palco dello stream: la finestra di Moonlight incorporata nella nostra (ADR 0015).
// Qualsiasi pannello aperto sopra (modale, menu) la «copre»: va nascosta, perché una
// finestra figlia resta sempre sopra la pagina web.
import { onBeforeUnmount, onMounted, reactive, readonly } from 'vue';
import { isTauri } from '@/ipc';
import { reportError } from './errors';

export interface StageRect {
  x: number;
  y: number;
  w: number;
  h: number;
}

interface StageState {
  /** La finestra del flusso è dentro Falco. */
  embedded: boolean;
  /** Ricerca della finestra in corso. */
  attaching: boolean;
  /** Moonlight non ha aperto una finestra incorporabile (macOS, errore). */
  external: boolean;
  fullscreen: boolean;
  /** Statistiche dettagliate visibili nella barra. */
  stats: boolean;
  covers: number;
  rect: StageRect | null;
}

const state = reactive<StageState>({
  embedded: false,
  attaching: false,
  external: false,
  fullscreen: false,
  stats: false,
  covers: 0,
  rect: null,
});

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T | null> {
  if (!isTauri()) return null;
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(cmd, args);
}

/** Cerca la finestra del flusso del processo e la porta dentro l'area indicata. */
export async function attach(pid: number, rect: StageRect): Promise<void> {
  if (state.embedded || state.attaching) return;
  state.rect = rect;
  if (!isTauri()) {
    // Nel browser il flusso è simulato
    state.embedded = true;
    return;
  }
  state.attaching = true;
  try {
    const ok = await invoke<boolean>('stream_attach', { pid, rect });
    state.embedded = ok === true;
    state.external = ok !== true;
    if (ok && state.rect) place(state.rect);
    if (ok) focusStream();
  } catch (e) {
    // Finestra non trovata: lo stream resta nella finestra di Moonlight; il motivo va nel report
    state.external = true;
    reportError(e, 'stream.attach');
  } finally {
    state.attaching = false;
  }
}

/** Tastiera allo stream incorporato (all'aggancio e quando la finestra di Falco torna attiva). */
export function focusStream(): void {
  if (state.embedded && state.covers === 0) void invoke('stream_focus');
}

let keeper: ReturnType<typeof setInterval> | undefined;

/**
 * Finché lo stream è visibile e niente lo copre, la tastiera resta a Moonlight: se il focus torna
 * alla pagina (succede a ogni clic sullo stream) lo si restituisce. Con un pannello aperto no,
 * così i campi di testo di Falco funzionano.
 */
export function keepStreamFocus(on: boolean): void {
  clearInterval(keeper);
  if (!on || !isTauri()) return;
  keeper = setInterval(() => {
    if (document.hasFocus() && state.embedded && state.covers === 0) void invoke('stream_focus');
  }, 300);
}

/** Nuova posizione del flusso; nascosto se qualcosa lo copre. */
export function place(rect: StageRect | null): void {
  if (rect) state.rect = rect;
  if (!state.embedded || !state.rect) return;
  if (state.covers > 0) void invoke('stream_detach', { hide: true });
  else void invoke('stream_place', { rect: state.rect });
}

/** Il palco non è visibile (altra sezione): il flusso si nasconde senza fermarsi. */
export function conceal(): void {
  if (state.embedded) void invoke('stream_detach', { hide: true });
}

/** Fine sessione: si dimentica la finestra e si esce dallo schermo intero. */
export async function resetStage(): Promise<void> {
  await invoke('stream_detach', { hide: false });
  if (state.fullscreen) await setFullscreen(false);
  state.embedded = false;
  state.external = false;
  state.attaching = false;
  state.stats = false;
}

// Un cambio di schermo intero alla volta: due clic ravvicinati non si accavallano
let fullscreenQueue: Promise<void> = Promise.resolve();

export function setFullscreen(on: boolean): Promise<void> {
  const run = async (): Promise<void> => {
    state.fullscreen = on;
    if (!isTauri()) return;
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    const win = getCurrentWindow();
    // Prima via il pulsantino, poi la finestra: non deve sparire mentre sta gestendo il clic
    if (!on) await invoke('pill', { show: false }).catch(() => {});
    if ((await win.isFullscreen()) !== on) await win.setFullscreen(on);
    if (on) await invoke('pill', { show: true });
  };
  fullscreenQueue = fullscreenQueue.then(run, run);
  return fullscreenQueue;
}

/** Statistiche: nello stream incorporato si accende l'overlay di Moonlight sopra il video. */
export function toggleStats(): void {
  state.stats = !state.stats;
  if (state.embedded) void invoke('stream_stats');
}

/** Da chiamare in ogni pannello che si apre sopra il palco. */
export function useStageCover(): void {
  onMounted(() => {
    state.covers++;
    place(null);
  });
  onBeforeUnmount(() => {
    state.covers = Math.max(0, state.covers - 1);
    // Il palco torna visibile al frame successivo, dopo la chiusura del pannello
    requestAnimationFrame(() => place(null));
  });
}

export const stage = readonly(state);
