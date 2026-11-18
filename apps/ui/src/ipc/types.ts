// Tipi dell'IPC, specchio di crates/falco-proto/src/ipc.rs e falco-store config.

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
  hasProfile: boolean;
  oauthConfigured: boolean;
  /** Sessione da ospite in corso. */
  session?: SessionInfo | null;
}

/** Sessione da ospite: da `session.started` o dallo snapshot. */
export interface SessionInfo {
  host: string;
  friendId: string | null;
  name: string;
  pid: number | null;
  startedAt: number;
  permissions: Permissions | null;
  app: string;
}

export interface SessionStats {
  rttMs: number;
  fps: number;
  bitrateKbps: number;
  path: 'direct' | 'relay' | string;
}

export type HostProfile = 'latency' | 'balanced' | 'slow' | 'quality' | 'custom';

export interface Recommendation {
  profile: Exclude<HostProfile, 'custom'>;
  reasons: string[];
  checks: DoctorCheck[];
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

export type FriendState = 'playing' | 'online' | 'busy' | 'offline' | 'pending';
export type Role = 'spettatore' | 'giocatore' | 'copilota' | 'fidato';

export interface FriendDevice {
  tsNodeId: string;
  tsIp: string;
  hostname: string;
  os: string;
  canHost: boolean;
}

export interface Friend {
  id: string;
  tag: string;
  name: string;
  accent: string;
  avatarUrl: string | null;
  state: FriendState;
  canJoin: boolean;
  slotsFree: number | null;
  favorite: boolean;
  lastSeen: number | null;
  role: Role | null;
  devices: readonly FriendDevice[];
}

export interface MyProfile {
  exists: boolean;
  id?: string;
  tag?: string;
  accent?: string;
  name?: string;
  avatarUrl?: string | null;
}

export interface Permissions {
  role: Role;
  view: boolean;
  audio: boolean;
  gamepads: number;
  keyboard: boolean;
  keyboardSystemKeys: boolean;
  mouse: boolean;
  launchApps: boolean;
  autoJoin: boolean;
  clipboard: boolean;
  endHostSession: boolean;
}

/** Falco Hub (ADR 0017): codice amico e link. */
export interface HubInfo {
  configured: boolean;
  code: string | null;
  friendLink: string | null;
  /** Rete Falco scelta come provider. */
  network: boolean;
}

export interface HubUser {
  id: string;
  name: string;
  code: string;
}

export interface FriendRequest {
  id: string;
  user: HubUser;
  createdAt: number;
}

export interface HubRequests {
  incoming: FriendRequest[];
  outgoing: FriendRequest[];
}

/** Richiesta accettata o link di stream aperto: invito da completare con l'handshake. */
export interface HubAccepted {
  invite: string;
  user: HubUser;
}

export interface StreamLink {
  code: string;
  url: string;
  expiresAt: number;
}

export interface InvitePreview {
  host: { id: string; name: string; tag: string; accent: string };
  role: Role;
  permissions: Permissions;
  expiresAt: number;
  needsTailscaleShare: boolean;
}

export interface InviteRow {
  tokenHash: string;
  role: Role;
  createdAt: number;
  expiresAt: number;
  usesLeft: number;
  needsApproval: boolean;
  active: boolean;
}
