import { invoke } from "@tauri-apps/api/core";
import type { Snapshot, Settings, LayoutDisplay } from "./types";
export const native = "__TAURI_INTERNALS__" in window;
const key = "hopping-cursor-interface-preview-v1";
const display = (id: string, width: number, height: number) => ({
  id,
  name: "Main display",
  bounds: { x: 0, y: 0, width, height },
  scale: 1,
  primary: true,
});
const sample: Snapshot = {
  local: {
    id: "local-preview",
    name: "Studio Mac",
    platform: "macos",
    displays: [display("main", 1920, 1080)],
  },
  public_key: "preview-public-key",
  fingerprint: "Preview identity — open the desktop app to pair real computers",
  settings: {
    device_name: "Studio Mac",
    port: 24810,
    clipboard_text: true,
    clipboard_images: true,
    clipboard_html: true,
    map_modifiers: true,
    auto_accept_files: false,
    receive_directory: "/Users/you/Downloads/Hopping Cursor",
    max_file_bytes: 10737418240,
    sensitivity: 1,
    edge_delay_ms: 80,
    paused: false,
    cursor_locked: false,
    game_mode: false,
  },
  peers: [
    {
      device: {
        id: "pc-preview",
        name: "Design PC",
        platform: "windows",
        displays: [display("main", 1920, 1080)],
      },
      address: "192.168.1.24:24810",
      public_key: "preview-key-pc",
      safety_code:
        "e20a 19f8 a130 3d09 c12b 7721 c441 d60f\n89af d171 2f05 a47c 19e0 81ad 192f a008",
      trusted: true,
      connected: true,
      latency_ms: null,
      last_error: null,
    },
    {
      device: {
        id: "laptop-preview",
        name: "Travel MacBook",
        platform: "macos",
        displays: [display("main", 1440, 900)],
      },
      address: "192.168.1.32:24810",
      public_key: "preview-key-laptop",
      safety_code:
        "d414 29e0 88a0 17aa 9237 cc19 ba2d 80d5\n0c71 918a a017 95f3 ca22 914e 8d61 812f",
      trusted: false,
      connected: false,
      latency_ms: null,
      last_error: "Approve on both computers",
    },
  ],
  layout: [
    {
      device_id: "local-preview",
      display_id: "main",
      rect: { x: 0, y: 0, width: 1920, height: 1080 },
      hidden: false,
    },
    {
      device_id: "pc-preview",
      display_id: "main",
      rect: { x: 1920, y: 0, width: 1920, height: 1080 },
      hidden: false,
    },
  ],
  profiles: [],
  hop_keys: [],
  peer_permissions: {
    "pc-preview": { input: true, clipboard: true, files: true },
  },
  transfers: [],
  permissions: {
    capture: true,
    injection: true,
    clipboard: true,
    explanation: "Sample permissions in interface preview",
  },
  active_device: "local-preview",
  listening_port: 24810,
  diagnostics: [
    "Interface preview loaded. No native input, network, or file sharing is running.",
  ],
  version: "0.1.1",
  session_available: true,
};
let preview: Snapshot;
try {
  preview =
    JSON.parse(localStorage.getItem(key) || "null") || structuredClone(sample);
} catch {
  preview = structuredClone(sample);
}
preview.profiles ??= [];
preview.hop_keys ??= [];
preview.peer_permissions ??= {};
let previewProfiles: Record<string, LayoutDisplay[]> = {};
try {
  previewProfiles = JSON.parse(localStorage.getItem(key + "-profiles") || "{}");
} catch {}
function persist() {
  localStorage.setItem(key, JSON.stringify(preview));
  localStorage.setItem(key + "-profiles", JSON.stringify(previewProfiles));
}
export async function call<T = void>(
  command: string,
  args: Record<string, unknown> = {},
): Promise<T> {
  if (native) return invoke<T>(command, args);
  switch (command) {
    case "snapshot":
      return structuredClone(preview) as T;
    case "update_settings":
      preview.settings = args.settings as Settings;
      preview.local.name = preview.settings.device_name;
      break;
    case "save_layout": {
      const layout = args.layout as LayoutDisplay[];
      if (!layout.some((d) => !d.hidden))
        throw Error("Keep at least one display visible");
      for (const a of layout)
        for (const b of layout)
          if (
            a !== b &&
            !a.hidden &&
            !b.hidden &&
            a.rect.x < b.rect.x + b.rect.width - 0.5 &&
            a.rect.x + a.rect.width > b.rect.x + 0.5 &&
            a.rect.y < b.rect.y + b.rect.height - 0.5 &&
            a.rect.y + a.rect.height > b.rect.y + 0.5
          )
            throw Error(
              "Visible displays cannot overlap; drag them edge to edge",
            );
      preview.layout = layout;
      break;
    }
    case "approve_peer": {
      const p = preview.peers.find((p) => p.device.id === args.id);
      if (p) {
        p.trusted = true;
        p.connected = true;
        p.last_error = null;
        if (!preview.layout.some((d) => d.device_id === p.device.id))
          preview.layout.push({
            device_id: p.device.id,
            display_id: "main",
            rect: { x: 3840, y: 0, width: 1440, height: 900 },
            hidden: false,
          });
      }
      break;
    }
    case "set_hop_key": {
      const binding = {
        device_id: String(args.deviceId),
        display_id: String(args.displayId),
        keys: args.keys as number[],
      };
      preview.hop_keys = preview.hop_keys.filter(
        (b) =>
          b.device_id !== binding.device_id ||
          b.display_id !== binding.display_id,
      );
      if (binding.keys.length) preview.hop_keys.push(binding);
      break;
    }
    case "jump_to_display":
      preview.active_device = String(args.deviceId);
      break;
    case "peer_permissions":
      preview.peer_permissions[String(args.id)] = {
        input: !!args.input,
        clipboard: !!args.clipboard,
        files: !!args.files,
      };
      break;
    case "save_profile":
      previewProfiles[String(args.name)] = structuredClone(preview.layout);
      preview.profiles = Object.keys(previewProfiles);
      break;
    case "load_profile":
      if (!previewProfiles[String(args.name)])
        throw Error("Unknown layout profile");
      preview.layout = structuredClone(previewProfiles[String(args.name)]);
      break;
    case "delete_profile":
      delete previewProfiles[String(args.name)];
      preview.profiles = Object.keys(previewProfiles);
      break;
    case "forget_peer":
      preview.peers = preview.peers.filter((p) => p.device.id !== args.id);
      preview.layout = preview.layout.filter((d) => d.device_id !== args.id);
      break;
    case "connect_address":
      throw Error(
        "Network connections run in the desktop app. This browser is an interface preview.",
      );
    case "send_files":
      throw Error("File transfers run in the desktop app.");
    case "respond_transfer":
    case "cancel_transfer":
      break;
    case "release_input":
      preview.active_device = preview.local.id;
      break;
    case "export_diagnostics":
      return JSON.stringify(
        {
          version: preview.version,
          mode: "interface preview",
          messages: preview.diagnostics,
        },
        null,
        2,
      ) as T;
    default:
      throw Error("This action requires the desktop app");
  }
  persist();
  return undefined as T;
}
