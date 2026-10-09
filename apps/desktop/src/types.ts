export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}
export interface Display {
  id: string;
  name: string;
  bounds: Rect;
  scale: number;
  primary: boolean;
}
export interface Device {
  id: string;
  name: string;
  platform: string;
  displays: Display[];
}
export interface LayoutDisplay {
  device_id: string;
  display_id: string;
  rect: Rect;
  hidden: boolean;
}
export interface Settings {
  device_name: string;
  port: number;
  clipboard_text: boolean;
  clipboard_images: boolean;
  clipboard_html: boolean;
  map_modifiers: boolean;
  auto_accept_files: boolean;
  receive_directory: string;
  max_file_bytes: number;
  sensitivity: number;
  edge_delay_ms: number;
  paused: boolean;
  cursor_locked: boolean;
  game_mode: boolean;
}
export interface Peer {
  device: Device;
  address: string;
  public_key: string | null;
  safety_code: string | null;
  trusted: boolean;
  connected: boolean;
  latency_ms: number | null;
  last_error: string | null;
}
export interface Transfer {
  id: string;
  peer_id: string;
  name: string;
  size: number;
  bytes: number;
  direction: string;
  state: string;
  error: string | null;
  saved_path: string | null;
}
export interface Snapshot {
  local: Device;
  public_key: string;
  fingerprint: string;
  settings: Settings;
  peers: Peer[];
  layout: LayoutDisplay[];
  profiles: string[];
  hop_keys: { device_id: string; display_id: string; keys: number[] }[];
  peer_permissions: Record<
    string,
    { input: boolean; clipboard: boolean; files: boolean }
  >;
  transfers: Transfer[];
  permissions: {
    capture: boolean;
    injection: boolean;
    clipboard: boolean;
    explanation: string;
  };
  active_device: string;
  listening_port: number;
  diagnostics: string[];
  version: string;
  session_available: boolean;
  suppressed_events?: number | null;
}
