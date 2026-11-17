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
  return (await getTransport()).call<T>(method, params);
}

export function isRpcError(e: unknown): e is RpcError {
  return typeof e === 'object' && e !== null && 'code' in e && 'message' in e;
}

export * from './types';
