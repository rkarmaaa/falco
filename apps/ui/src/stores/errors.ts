// Centro errori: ogni errore ha un messaggio in tre parti, «Copia» e «Debug» (§13).
import { reactive, readonly } from 'vue';
import { i18n } from '@/i18n';
import {
  CORE_UNREACHABLE,
  HOST_NOT_READY,
  NOT_IMPLEMENTED,
  SESSION_FAILED,
  getTransport,
  isRpcError,
  type RpcError,
} from '@/ipc';

export interface AppError {
  id: number;
  at: string;
  /** Cosa stava facendo l'utente o Falco (chiave o testo). */
  context: string;
  /** Messaggio per l'utente: cosa, perché, cosa fare. */
  friendly: string;
  technical: {
    source: string;
    code?: number;
    message: string;
    data?: unknown;
    stack?: string;
  };
}

interface ErrorState {
  toasts: AppError[];
  history: AppError[];
  debug: AppError | null;
}

const state = reactive<ErrorState>({ toasts: [], history: [], debug: null });
let nextId = 1;

const t = (key: string, args?: Record<string, unknown>) => i18n.global.t(key, args ?? {});
const te = (key: string) => i18n.global.te(key);

/** Messaggio in tre parti dal codice dell'errore. */
export function friendlyMessage(e: unknown, ctx: { name?: string } = {}): string {
  if (isRpcError(e)) {
    const data = (e.data ?? {}) as Record<string, unknown>;
    switch (e.code) {
      case CORE_UNREACHABLE:
        return `${t('core.offlineTitle')} ${t('core.offlineWhy')} ${t('core.offlineFix')}`;
      case SESSION_FAILED:
        if (typeof data.reason === 'string' && te(`joinError.${data.reason}`))
          return t(`joinError.${data.reason}`, { name: ctx.name ?? t('errors.theHost') });
        return t('errors.session', { why: e.message });
      case HOST_NOT_READY:
        return t('errors.hostNotReady');
      case NOT_IMPLEMENTED:
        return t('errors.notImplemented', { milestone: String(data.milestone ?? '') });
      case -32002:
        return t('errors.config', { key: String(data.key ?? ''), allowed: String(data.allowed ?? '') });
      case -32602:
        return t('errors.invalidParams', { why: e.message });
      case -32601:
        return t('errors.unknownMethod');
    }
    return t('errors.generic', { why: e.message });
  }
  const msg = e instanceof Error ? e.message : String(e);
  return t('errors.generic', { why: msg });
}

/** Crea un errore senza mostrarlo (per gli avvisi in linea). */
export function makeError(e: unknown, context: string, opts: { friendly?: string; source?: string; name?: string } = {}): AppError {
  const rpc = isRpcError(e) ? (e as RpcError & { method?: string }) : null;
  const err: AppError = {
    id: nextId++,
    at: new Date().toISOString(),
    context,
    friendly: opts.friendly ?? friendlyMessage(e, { name: opts.name }),
    technical: {
      source: opts.source ?? rpc?.method ?? 'ui',
      code: rpc?.code,
      message: rpc ? rpc.message : e instanceof Error ? e.message : String(e),
      data: rpc?.data ?? undefined,
      stack: e instanceof Error ? e.stack : undefined,
    },
  };
  state.history.unshift(err);
  state.history.splice(50);
  return err;
}

/** Mostra l'errore come avviso in basso a destra. */
export function reportError(e: unknown, context: string, opts: { friendly?: string; source?: string; name?: string } = {}): AppError {
  const err = makeError(e, context, opts);
  state.toasts.unshift(err);
  state.toasts.splice(3);
  return err;
}

/** Esegue un'azione e mostra l'eventuale errore. */
export async function attempt<T>(context: string, fn: () => Promise<T>, opts: { name?: string } = {}): Promise<T | undefined> {
  try {
    return await fn();
  } catch (e) {
    reportError(e, context, opts);
    return undefined;
  }
}

export function dismiss(id: number): void {
  state.toasts = state.toasts.filter((e) => e.id !== id);
}

export function openDebug(err: AppError): void {
  state.debug = err;
}

export function closeDebug(): void {
  state.debug = null;
}

/** Riassunto copiabile di un errore. */
export function describe(err: AppError): string {
  const tech = err.technical;
  return [
    `Falco — ${t('errors.reportTitle')}`,
    `${t('debug.when')}: ${err.at}`,
    `${t('debug.context')}: ${err.context}`,
    `${t('debug.message')}: ${err.friendly}`,
    `${t('debug.source')}: ${tech.source}${tech.code !== undefined ? ` (${tech.code})` : ''}`,
    `${t('debug.technical')}: ${tech.message}`,
    tech.data ? `data: ${JSON.stringify(tech.data)}` : '',
  ]
    .filter(Boolean)
    .join('\n');
}

/** Errori delle attività di sfondo inviati dal core. */
export async function initErrors(): Promise<void> {
  const transport = await getTransport();
  transport.onEvent((e) => {
    if (e.method !== 'error.occurred') return;
    const p = e.params as { source: string; message: string; data?: unknown };
    const key = `errors.background.${p.source.replace('.', '_')}`;
    reportError({ code: 0, message: p.message, data: p.data }, p.source, {
      source: p.source,
      friendly: te(key) ? t(key, { why: p.message }) : t('errors.generic', { why: p.message }),
    });
  });
  // Errori imprevisti: nessun errore resta senza messaggio
  window.addEventListener('unhandledrejection', (ev) => {
    reportError(ev.reason, t('errors.unexpected'), { source: 'ui' });
  });
  window.addEventListener('error', (ev) => {
    reportError(ev.error ?? ev.message, t('errors.unexpected'), { source: 'ui' });
  });
}

export const errors = readonly(state);
