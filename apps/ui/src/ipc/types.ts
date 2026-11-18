// Tipi dell'IPC, specchio di crates/falo-proto/src/ipc.rs e falo-store config.

export type Status = 'online' | 'busy' | 'invisible';
export type TailscaleState = 'missing' | 'stopped' | 'needsLogin' | 'running';

export interface HostState {
  state: 'stopped' | 'starting' | 'ready' | 'error';
  message?: string;
}

export interface PendingPairing {
  id: string;
  name: string;
  address: string;
}

export interface PairedClient {
  uuid: string;
  name: string;
  enabled: boolean;
}

export interface StateSnapshot {
  version: string;
  host: HostState;
  milestone: string;
  status: Status;
  hostAvailable: boolean;
  streaming: boolean;
  network: {
    provider: string;
    tailscale: TailscaleState;
    selfIps: string[];
    exitNodeActive: boolean;
  };
  friendsCount: number;
}

export interface RpcError {
  code: number;
  message: string;
  data?: Record<string, unknown> | null;
}

export interface CoreEvent {
  method: string;
  params: unknown;
}

export type CheckLevel = 'ok' | 'warn' | 'fail' | 'pending';

export interface DoctorCheck {
  id: string;
  level: CheckLevel;
  value: string | null;
  action: string | null;
}

export type ApplyAt = 'now' | 'nextSession' | 'hostRestart';

export interface FaloConfig {
  general: {
    language: 'it' | 'en';
    updates: 'auto' | 'notify' | 'off';
    theme: 'system' | 'dark' | 'light';
    start_with_system: boolean;
    start_minimized: boolean;
    close_to_tray: boolean;
  };
  profile: { status: Status };
  stream: Record<string, unknown>;
  host: { max_guests: number; encoder: string; default_role: string } & Record<string, unknown>;
  network: { provider: string; presence_port: number; fec_percent: number } & Record<string, unknown>;
  performance: Record<string, unknown>;
  advanced: Record<string, unknown>;
}

export const CORE_UNREACHABLE = -32099;
export const NOT_IMPLEMENTED = -32001;
export const HOST_NOT_READY = -32003;
export const SESSION_FAILED = -32004;
