import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Transport } from './index';
import type { CoreEvent } from './types';

export function createTauriTransport(): Transport {
  return {
    call: (method, params) => invoke('rpc', { method, params: params ?? null }),
    onEvent: (cb) => void listen<CoreEvent>('core://event', (e) => cb(e.payload)),
    onConnection: (cb) => {
      void listen('core://connected', () => cb(true));
      void listen('core://disconnected', () => cb(false));
    },
    connected: () => invoke<boolean>('core_connected'),
  };
}
