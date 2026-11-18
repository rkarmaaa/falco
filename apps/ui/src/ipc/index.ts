// Trasporto verso il core: Tauri nella finestra vera, simulato nel browser
// (sviluppo con `npm run dev`, test visivi con Playwright).
import type { CoreEvent, RpcError } from './types';

export interface Transport {
  call<T>(method: string, params?: unknown): Promise<T>;
  onEvent(cb: (e: CoreEvent) => void): void;
  onConnection(cb: (connected: boolean) => void): void;
  connected(): Promise<boolean>;
}

export const isTauri = (): boolean => '__TAURI_INTERNALS__' in window;

let transport: Promise<Transport> | null = null;

export function getTransport(): Promise<Transport> {
  transport ??= isTauri()
    ? import('./tauri').then((m) => m.createTauriTransport())
    : import('./mock').then((m) => m.createMockTransport());
  return transport;
}

export async function rpc<T = unknown>(method: string, params?: unknown): Promise<T> {
  try {
    return await (await getTransport()).call<T>(method, params);
  } catch (e) {
    // Il metodo serve alla schermata «Debug»; i parametri no (possono contenere segreti)
    if (isRpcError(e)) throw Object.assign({}, e, { method });
    throw { code: -32603, message: e instanceof Error ? e.message : String(e), method };
  }
}

export function isRpcError(e: unknown): e is RpcError {
  return typeof e === 'object' && e !== null && 'code' in e && 'message' in e;
}

export * from './types';
