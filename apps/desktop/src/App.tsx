import { useCallback, useEffect, useId, useRef, useState } from "react";
import {
  LayoutDashboard,
  Monitor,
  ArrowDownToLine,
  Settings2,
  Activity,
  MousePointer2,
  Keyboard,
  Clipboard,
  ShieldCheck,
  Plus,
  Pause,
  Play,
  RotateCcw,
  Check,
  X,
  Wifi,
  LockKeyhole,
  ArrowUpRight,
  File,
  Upload,
  EyeOff,
  Eye,
  Laptop,
  CircleHelp,
  Unplug,
  ChevronRight,
  Download,
  Copy,
} from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { call, native } from "./api";
import type {
  Snapshot,
  Peer,
  Settings,
  LayoutDisplay,
  Device,
  Transfer,
} from "./types";

type Page =
  "workspace" | "computers" | "transfers" | "preferences" | "diagnostics";
const nav = [
  ["workspace", "Workspace", LayoutDashboard],
  ["computers", "Computers", Monitor],
  ["transfers", "File transfers", ArrowDownToLine],
  ["preferences", "Preferences", Settings2],
  ["diagnostics", "Diagnostics", Activity],
] as const;
const bytes = (n: number) =>
  n < 1024
    ? `${n} B`
    : n < 1048576
      ? `${(n / 1024).toFixed(1)} KB`
      : n < 1073741824
        ? `${(n / 1048576).toFixed(1)} MB`
        : `${(n / 1073741824).toFixed(1)} GB`;
const platform = (s: string) =>
  s === "macos"
    ? "macOS"
    : s === "windows"
      ? "Windows"
      : s === "headless"
        ? "Headless"
        : "Computer";
function Mark() {
  return (
    <svg width="34" height="34" viewBox="0 0 34 34" aria-hidden="true">
      <rect x="1" y="1" width="32" height="32" rx="10" fill="currentColor" />
      <path d="M10 9v15l4.6-4.7 3.1 6 3-1.6-3.2-5.8H24L10 9Z" fill="white" />
      <path
        d="M22 9h4v4M26 9l-6 6"
        fill="none"
        stroke="white"
        strokeWidth="1.5"
        strokeLinecap="round"
      />
    </svg>
  );
}
function OsIcon({ os }: { os: string }) {
  return os === "macos" ? <Laptop size={19} /> : <Monitor size={19} />;
}
function Badge({
  children,
  tone = "neutral",
}: {
  children: React.ReactNode;
  tone?: string;
}) {
  return <span className={`badge ${tone}`}>{children}</span>;
}
function Toggle({
  label,
  description,
  on,
  change,
}: {
  label: string;
  description: string;
  on: boolean;
  change: () => void;
}) {
  return (
    <div className="setting-row">
      <div>
        <strong>{label}</strong>
        <p>{description}</p>
      </div>
      <button
        type="button"
        role="switch"
        aria-checked={on}
        aria-label={label}
        className={`switch ${on ? "on" : ""}`}
        onClick={change}
      >
        <span />
      </button>
    </div>
  );
}

export default function App() {
  const [data, setData] = useState<Snapshot | null>(null),
    [page, setPage] = useState<Page>("workspace");
  const [failure, setFailure] = useState(""),
    [toast, setToast] = useState(""),
    [pair, setPair] = useState<Peer | null>(null),
    [add, setAdd] = useState(false),
    [address, setAddress] = useState("");
  const [matched, setMatched] = useState(false),
    [connecting, setConnecting] = useState(false),
    [filePeer, setFilePeer] = useState(""),
    [permissionPeer, setPermissionPeer] = useState<Peer | null>(null),
    [dropPaths, setDropPaths] = useState<string[]>([]);
  const toastTimer = useRef<ReturnType<typeof setTimeout> | undefined>(
    undefined,
  );
  const refresh = useCallback(async () => {
    try {
      const next = await call<Snapshot>("snapshot");
      setData(next);
      setFailure("");
      return next;
    } catch (e) {
      setFailure(String(e));
      return null;
    }
  }, []);
  useEffect(() => {
    void refresh();
    const interval = setInterval(() => void refresh(), 1000);
    return () => clearInterval(interval);
  }, [refresh]);
  const notify = useCallback((message: string) => {
    setToast(message);
    clearTimeout(toastTimer.current);
    toastTimer.current = setTimeout(() => setToast(""), 5500);
  }, []);
  const run = async (action: () => Promise<unknown>, success?: string) => {
    try {
      await action();
      await refresh();
      if (success) notify(success);
      return true;
    } catch (e) {
      notify(String(e));
      await refresh();
      return false;
    }
  };
  useEffect(() => {
    if (!native) return;
    const promise = getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === "drop") {
        setDropPaths(event.payload.paths);
        setPage("transfers");
      }
    });
    return () => {
      void promise.then((unlisten) => unlisten());
    };
  }, []);
  const change = async (patch: Partial<Settings>) => {
    if (data)
      await run(
        () =>
          call("update_settings", { settings: { ...data.settings, ...patch } }),
        "Preferences saved",
      );
  };
  const beginPair = async (p: Peer) => {
    if (p.public_key) {
      setPair(p);
      setMatched(false);
      return;
    }
    setConnecting(true);
    try {
      await call("connect_address", { address: p.address });
    } catch (e) {
      if (!String(e).includes("Pairing pending")) notify(String(e));
    }
    const next = await refresh();
    const actual = next?.peers.find((peer) => peer.device.id === p.device.id);
    if (actual?.public_key) {
      setPair(actual);
      setMatched(false);
    }
    setConnecting(false);
  };
  const sendFiles = async (paths?: string[]) => {
    if (!filePeer) {
      notify("Choose a paired computer first");
      return;
    }
    if (!native) {
      notify("Open the desktop app to send real files");
      return;
    }
    const selected =
      paths ||
      (await open({
        multiple: true,
        directory: false,
        title: "Choose files to send",
      }));
    if (!selected) return;
    setDropPaths([]);
    await run(
      () =>
        call("send_files", {
          peer: filePeer,
          paths: Array.isArray(selected) ? selected : [selected],
        }),
      "Files delivered and verified",
    );
  };
  if (!data)
    return (
      <div className="startup">
        <Mark />
        <h1>The Hopping Cursor</h1>
        <p>{failure || "Starting your local workspace…"}</p>
        {failure && <button onClick={() => void refresh()}>Check again</button>}
        {native && <button onClick={() => void call("quit_app")}>Quit application</button>}
      </div>
    );
  const connected = data.peers.filter((p) => p.connected),
    active =
      data.local.id === data.active_device
        ? data.local.name
        : data.peers.find((p) => p.device.id === data.active_device)?.device
            .name || "Remote computer";
  const pending = data.transfers.filter((t) => t.state === "offered");
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <a
          className="brand"
          href="#workspace"
          onClick={() => setPage("workspace")}
          aria-label="The Hopping Cursor workspace"
        >
          <Mark />
          <span>
            The Hopping
            <br />
            <b>Cursor</b>
          </span>
        </a>
        <div className="workspace-label">
          <span className="workspace-avatar">
            <Monitor size={17} />
          </span>
          <div>
            <strong>My desk</strong>
            <small>Local workspace</small>
          </div>
          <LockKeyhole size={14} />
        </div>
        <nav aria-label="Main navigation">
          {nav.map(([id, label, Icon]) => (
            <button
              aria-label={label}
              key={id}
              className={page === id ? "selected" : ""}
              onClick={() => setPage(id)}
            >
              <Icon size={19} />
              <span>{label}</span>
              {id === "computers" && data.peers.length > 0 && (
                <small>{data.peers.length + 1}</small>
              )}
              {id === "transfers" && pending.length > 0 && (
                <small className="attention-count">{pending.length}</small>
              )}
            </button>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <div className="network-note">
            <ShieldCheck size={19} />
            <div>
              <strong>Just your local network</strong>
              <p>Encrypted between paired computers.</p>
            </div>
          </div>
          <button className="help-link" onClick={() => setPage("diagnostics")}>
            <CircleHelp size={16} />
            Connection help
            <ChevronRight size={14} />
          </button>
          <div className="version">
            The Hopping Cursor <span>v{data.version}</span>
          </div>
        </div>
      </aside>
      <main className="main">
        {!native && (
          <div className="preview-banner">
            <Monitor size={15} />
            <span>
              Interface preview with sample computers. Real sharing runs in the
              desktop app.
            </span>
          </div>
        )}
        <header className="topbar">
          <span>
            <span
              className={`status-dot ${data.settings.paused ? "amber" : ""}`}
            />
            {data.settings.paused ? "Sharing paused" : "Workspace ready"}
          </span>
          <span className="local-label">
            <OsIcon os={data.local.platform} />
            {data.local.name}
            <Badge>This computer</Badge>
          </span>
        </header>
        <div className="page-content">
          <div className="page-heading">
            <div>
              <h1>
                {page === "workspace"
                  ? "Your desk, connected."
                  : page === "computers"
                    ? "Your computers"
                    : page === "transfers"
                      ? "Move files across your desk"
                      : page === "preferences"
                        ? "Make it feel like your desk"
                        : "Connection diagnostics"}
              </h1>
              <p>
                {page === "workspace"
                  ? "Use either mouse. Your keyboards follow the shared cursor."
                  : page === "computers"
                    ? "Discover, pair, and manage the computers on your local network."
                    : page === "transfers"
                      ? "Direct transfers, with permission and progress at both ends."
                      : page === "preferences"
                        ? "Choose what follows you from one computer to the next."
                        : "Understand your connection without recording what you type."}
              </p>
            </div>
            <div className="heading-actions">
              <button
                className="button secondary"
                onClick={() =>
                  void run(
                    () => call("release_input"),
                    "Input returned to this computer",
                  )
                }
                title="Emergency shortcut: Ctrl + Alt + Esc"
              >
                <RotateCcw size={16} />
                Bring input here
              </button>
              <button
                className={`button ${data.settings.paused ? "primary" : "secondary"}`}
                onClick={() => void change({ paused: !data.settings.paused })}
              >
                {data.settings.paused ? (
                  <Play size={16} />
                ) : (
                  <Pause size={16} />
                )}{" "}
                {data.settings.paused ? "Resume" : "Pause"}
              </button>
            </div>
          </div>
          {failure && (
            <div className="notice danger" role="alert">
              {failure}
              <button onClick={() => void refresh()}>Retry</button>
            </div>
          )}
          {page !== "workspace" && (data.layout_offers || []).length > 0 && (
            <div className="notice" role="status">
              A paired computer sent a screen arrangement.
              <button
                className="text-button"
                onClick={() => setPage("workspace")}
              >
                Review arrangement
              </button>
            </div>
          )}
          {native && !data.session_available && (
            <div className="notice" role="status">
              Sharing is suspended while the OS input session is protected.
              Return to the normal desktop or leave secure keyboard entry to
              resume.
            </div>
          )}
          {native &&
            (!data.permissions.capture || !data.permissions.injection) && (
              <div className="notice">
                <ShieldCheck size={19} />
                <div>
                  <strong>Finish input permissions</strong>
                  <p>{data.permissions.explanation}</p>
                </div>
                <button
                  className="button secondary"
                  onClick={() =>
                    void run(
                      () => call("request_input_permissions"),
                      "Permission status refreshed. Approve the system prompts, then retry or restart if requested.",
                    )
                  }
                >
                  Grant permissions / Retry
                </button>
              </div>
            )}
          {page === "workspace" && (
            <>
              <div className="workspace-summary">
                <div>
                  <span className="summary-icon">
                    <MousePointer2 size={21} />
                  </span>
                  <div>
                    <small>Input is on</small>
                    <strong>{active}</strong>
                  </div>
                  <Badge
                    tone={
                      data.settings.paused ||
                      !data.permissions.capture ||
                      !data.permissions.injection
                        ? "warning"
                        : "success"
                    }
                  >
                    {data.settings.paused
                      ? "Paused"
                      : !data.permissions.capture || !data.permissions.injection
                        ? "Permissions needed"
                        : "Ready"}
                  </Badge>
                </div>
                <div>
                  <span className="summary-icon">
                    <Monitor size={21} />
                  </span>
                  <div>
                    <small>Paired and connected</small>
                    <strong>
                      {connected.length} computer
                      {connected.length === 1 ? "" : "s"}
                    </strong>
                  </div>
                </div>
                <div>
                  <span className="summary-icon">
                    <ShieldCheck size={21} />
                  </span>
                  <div>
                    <small>Connection privacy</small>
                    <strong>Encrypted over LAN</strong>
                  </div>
                </div>
              </div>
              <Arrangement
                data={data}
                save={(layout) =>
                  run(
                    () => call("save_layout", { layout }),
                    "Display arrangement saved",
                  )
                }
                action={(command, args, message) =>
                  run(() => call(command, args), message)
                }
                add={() => setAdd(true)}
              />
              <div className="workspace-bottom">
                <section className="panel computers-panel">
                  <div className="section-heading">
                    <h2>On your desk</h2>
                    <button
                      className="text-button"
                      onClick={() => setPage("computers")}
                    >
                      Manage computers
                      <ChevronRight size={15} />
                    </button>
                  </div>
                  <ComputerRow device={data.local} local />
                  <>
                    {connected.map((p) => (
                      <ComputerRow
                        key={p.device.id}
                        device={p.device}
                        peer={p}
                      />
                    ))}
                  </>
                  {!connected.length && (
                    <div className="inline-empty">
                      <Wifi size={20} />
                      <p>
                        Install the app on another computer, then pair it here.
                      </p>
                      <button
                        className="text-button"
                        onClick={() => setAdd(true)}
                      >
                        Add a computer
                      </button>
                    </div>
                  )}
                </section>
                <section className="panel sharing-panel">
                  <h2>What follows your cursor</h2>
                  <Feature
                    icon={Keyboard}
                    title="Mouse & keyboard"
                    description="Input moves to the active computer."
                    on={!data.settings.paused}
                  />
                  <Feature
                    icon={Clipboard}
                    title="Shared clipboard"
                    description="Text, formatting, and images."
                    on={
                      data.settings.clipboard_text ||
                      data.settings.clipboard_images
                    }
                  />
                  <Feature
                    icon={File}
                    title="Local file transfers"
                    description="Files stay on your network."
                    on
                  />
                  <button
                    className="text-button"
                    onClick={() => setPage("preferences")}
                  >
                    Adjust sharing
                    <ChevronRight size={15} />
                  </button>
                </section>
              </div>
              <div className="shortcut-tip">
                <Keyboard size={16} />
                <span>
                  Need your cursor back? Press <kbd>Ctrl</kbd> + <kbd>Alt</kbd>{" "}
                  + <kbd>Esc</kbd> on the source computer.
                </span>
              </div>
            </>
          )}
          {page === "computers" && (
            <>
              <div className="section-heading section-space">
                <h2>{data.peers.length + 1} computers in this workspace</h2>
                <button className="button primary" onClick={() => setAdd(true)}>
                  <Plus size={17} />
                  Add computer
                </button>
              </div>
              <section className="panel device-list">
                <ComputerRow device={data.local} local />
                <>
                  {data.peers.map((p) => (
                    <div className="device-detail" key={p.device.id}>
                      <ComputerRow device={p.device} peer={p} />
                      <div className="device-actions">
                        {!p.trusted ? (
                          <button
                            className="button primary"
                            disabled={connecting}
                            onClick={() => void beginPair(p)}
                          >
                            <ShieldCheck size={16} />
                            Pair computer
                          </button>
                        ) : (
                          <>
                            <button
                              className="button secondary"
                              onClick={() => {
                                setFilePeer(p.device.id);
                                setPage("transfers");
                              }}
                              disabled={!p.connected}
                            >
                              <Upload size={16} />
                              Send files
                            </button>
                            <button
                              className="icon-button"
                              aria-label={`Sharing permissions for ${p.device.name}`}
                              onClick={() => setPermissionPeer(p)}
                            >
                              <Settings2 size={17} />
                            </button>
                            <button
                              className="icon-button"
                              aria-label={`Unpair ${p.device.name}`}
                              onClick={() => {
                                if (
                                  window.confirm(
                                    `Unpair ${p.device.name}? Its input and data access will be removed.`,
                                  )
                                )
                                  void run(
                                    () =>
                                      call("forget_peer", { id: p.device.id }),
                                    "Computer unpaired",
                                  );
                              }}
                            >
                              <Unplug size={17} />
                            </button>
                          </>
                        )}
                      </div>
                      {p.last_error && (
                        <p className="device-error">{p.last_error}</p>
                      )}
                    </div>
                  ))}
                </>
              </section>
              <div className="explanation">
                <ShieldCheck size={24} />
                <div>
                  <h3>Discovery finds computers. Pairing grants access.</h3>
                  <p>
                    Compare the safety code on both screens before approving.
                    Nearby computers cannot control your input or read your
                    clipboard until you pair them.
                  </p>
                </div>
              </div>
            </>
          )}
          {page === "transfers" && (
            <>
              <section className="panel transfer-picker">
                <div>
                  <Upload size={29} />
                  <h2>Send a file, without the cloud.</h2>
                  <p>
                    Choose a paired computer and select files, drop files into
                    this window, or send files copied in Finder or Explorer.
                  </p>
                </div>
                <div className="transfer-controls">
                  <label htmlFor="recipient">Send to</label>
                  <select
                    id="recipient"
                    value={filePeer}
                    onChange={(e) => setFilePeer(e.target.value)}
                  >
                    <option value="">Choose a computer</option>
                    {connected.map((p) => (
                      <option key={p.device.id} value={p.device.id}>
                        {p.device.name}
                      </option>
                    ))}
                  </select>
                  <button
                    className="button primary"
                    onClick={() => void sendFiles()}
                  >
                    <Plus size={16} />
                    Choose files
                  </button>
                  <button
                    className="button secondary"
                    disabled={!filePeer || !native}
                    onClick={() =>
                      void run(
                        () => call("send_copied_files", { peer: filePeer }),
                        "Copied files sent and verified",
                      )
                    }
                  >
                    Send copied files
                  </button>
                </div>
                {dropPaths.length > 0 && (
                  <div className="notice">
                    <File size={17} />
                    <span>
                      {dropPaths.length} dropped file
                      {dropPaths.length === 1 ? "" : "s"} ready to send
                    </span>
                    <button
                      className="button primary"
                      onClick={() => void sendFiles(dropPaths)}
                    >
                      Send dropped files
                    </button>
                  </div>
                )}
              </section>
              <section className="panel">
                <div className="section-heading">
                  <h2>Transfer activity</h2>
                  <Badge>{data.transfers.length}</Badge>
                </div>
                {data.transfers.length === 0 ? (
                  <div className="empty-state">
                    <ArrowDownToLine size={33} />
                    <h3>Your next transfer starts here.</h3>
                    <p>
                      Incoming files ask for your approval. Completed files go
                      to your receiving folder.
                    </p>
                  </div>
                ) : (
                  data.transfers.map((t) => (
                    <TransferRow
                      key={t.id}
                      t={t}
                      accept={(accepted) =>
                        void run(() =>
                          call("respond_transfer", { id: t.id, accepted }),
                        )
                      }
                      cancel={() =>
                        void run(() => call("cancel_transfer", { id: t.id }))
                      }
                      copy={() =>
                        void run(
                          () => call("copy_received_file", { id: t.id }),
                          "File copied. Paste in Finder or Explorer to copy it to another folder.",
                        )
                      }
                    />
                  ))
                )}
                <div className="folder-note">
                  <Download size={15} />
                  <span>
                    Receiving folder: {data.settings.receive_directory}
                  </span>
                  <button
                    className="text-button"
                    onClick={() => setPage("preferences")}
                  >
                    Change
                  </button>
                </div>
              </section>
              <p className="footnote">
                Every transfer uses a separate encrypted connection and verifies
                the file before saving. Zip folders before sending. Cross-screen
                Finder/Explorer dragging is still planned.
              </p>
            </>
          )}
          {page === "preferences" && (
            <Preferences
              settings={data.settings}
              change={(patch) => void change(patch)}
              chooseFolder={async () => {
                if (!native) {
                  notify("Folder selection runs in the desktop app");
                  return;
                }
                const selected = await open({
                  directory: true,
                  multiple: false,
                });
                if (typeof selected === "string")
                  await change({ receive_directory: selected });
              }}
            />
          )}
          {page === "diagnostics" && (
            <>
              <section className="panel">
                <div className="section-heading">
                  <h2>Engine health</h2>
                  <button
                    className="button secondary"
                    onClick={() => void refresh()}
                  >
                    <RotateCcw size={16} />
                    Recheck
                  </button>
                </div>
                <div className="health-grid">
                  <Health label="Input capture" ok={data.permissions.capture} />
                  <Health
                    label="Input injection"
                    ok={data.permissions.injection}
                  />
                  <Health
                    label="Clipboard access"
                    ok={data.permissions.clipboard}
                  />
                </div>
                <p className="footnote">{data.permissions.explanation}</p>
                <dl className="diagnostic-values">
                  {data.suppressed_events != null && (
                    <div>
                      <dt>Locally suppressed input events</dt>
                      <dd>{data.suppressed_events}</dd>
                    </div>
                  )}
                  <div>
                    <dt>Mouse controller</dt>
                    <dd>
                      {data.controller_device === data.local.id
                        ? data.local.name
                        : data.peers.find(
                            (p) => p.device.id === data.controller_device,
                          )?.device.name || "This computer"}
                    </dd>
                  </div>
                  <div>
                    <dt>Cursor destination</dt>
                    <dd>
                      {data.active_device === data.local.id
                        ? data.local.name
                        : data.peers.find(
                            (p) => p.device.id === data.active_device,
                          )?.device.name || "Unavailable"}
                    </dd>
                  </div>
                  <div>
                    <dt>Local cursor</dt>
                    <dd>
                      {data.cursor_visible === false
                        ? "Hidden while input is elsewhere"
                        : "Visible"}
                    </dd>
                  </div>
                  <div>
                    <dt>Listening port</dt>
                    <dd>TCP {data.listening_port}</dd>
                  </div>
                  <div>
                    <dt>Discovery</dt>
                    <dd>mDNS · UDP 5353</dd>
                  </div>
                  <div>
                    <dt>Encryption</dt>
                    <dd>Noise XX / ChaCha20-Poly1305</dd>
                  </div>
                  <div>
                    <dt>Connected computers</dt>
                    <dd>{connected.length}</dd>
                  </div>
                  {connected
                    .filter((p) => p.latency_ms !== null)
                    .map((p) => (
                      <div key={p.device.id}>
                        <dt>{p.device.name} network round trip</dt>
                        <dd>{p.latency_ms?.toFixed(2)} ms</dd>
                      </div>
                    ))}
                </dl>
                <p className="footnote">
                  Network round trip is measured from live heartbeats. It does
                  not measure end-to-end cursor latency.
                </p>
              </section>
              <section className="panel">
                <div className="section-heading">
                  <h2>Local public identity</h2>
                  <button
                    className="text-button"
                    onClick={() =>
                      void run(
                        () => navigator.clipboard.writeText(data.fingerprint),
                        "Fingerprint copied",
                      )
                    }
                  >
                    <Copy size={15} />
                    Copy fingerprint
                  </button>
                </div>
                <code className="fingerprint">{data.fingerprint}</code>
                <p className="footnote">
                  This identifies your computer. Your private key is never
                  displayed or included in diagnostics.
                </p>
              </section>
              <section className="panel">
                <div className="section-heading">
                  <h2>Recent engine messages</h2>
                  <button
                    className="button secondary"
                    onClick={() =>
                      void run(async () => {
                        const text = await call<string>("export_diagnostics");
                        const url = URL.createObjectURL(
                          new Blob([text], { type: "application/json" }),
                        );
                        const a = document.createElement("a");
                        a.href = url;
                        a.download = "hopping-cursor-diagnostics.json";
                        a.click();
                        setTimeout(() => URL.revokeObjectURL(url), 1000);
                      }, "Diagnostics exported")
                    }
                  >
                    <Download size={16} />
                    Export diagnostics
                  </button>
                </div>
                <div className="log-list">
                  {data.diagnostics.map((line, i) => (
                    <p key={i}>{line}</p>
                  ))}
                </div>
              </section>
              <div className="explanation">
                <Wifi size={24} />
                <div>
                  <h3>If your computers cannot find each other</h3>
                  <p>
                    Use the same private network. Allow TCP{" "}
                    {data.listening_port} and mDNS through your firewall. Guest
                    Wi-Fi and workplace networks may isolate devices; try
                    connecting by local IP address.
                  </p>
                </div>
              </div>
            </>
          )}
        </div>
      </main>
      {toast && (
        <div className="toast" role="status">
          <span>{toast}</span>
          <button
            aria-label="Dismiss notification"
            onClick={() => setToast("")}
          >
            <X size={16} />
          </button>
        </div>
      )}
      {add && (
        <Dialog title="Add a computer" close={() => setAdd(false)}>
          <p>
            Install The Hopping Cursor on the other computer and use the same
            local network. It should appear in Computers automatically.
          </p>
          <label className="field-label" htmlFor="address">
            Connect by local IP address
          </label>
          <input
            id="address"
            value={address}
            onChange={(e) => setAddress(e.target.value)}
            placeholder="192.168.1.20:24810"
          />
          <p className="footnote">
            Use this when automatic discovery is blocked. The encrypted
            handshake will reveal the computer’s safety code.
          </p>
          <div className="dialog-actions">
            <button
              className="button secondary"
              onClick={() => {
                setPage("computers");
                setAdd(false);
              }}
            >
              View nearby computers
            </button>
            <button
              className="button primary"
              disabled={!address.trim() || connecting}
              onClick={async () => {
                setConnecting(true);
                try {
                  await call("connect_address", { address });
                } catch (e) {
                  notify(String(e));
                }
                await refresh();
                setConnecting(false);
                setPage("computers");
                setAdd(false);
              }}
            >
              {connecting ? "Connecting…" : "Connect"}
            </button>
          </div>
        </Dialog>
      )}
      {pair && (
        <Dialog
          title={`Pair with ${pair.device.name}`}
          close={() => setPair(null)}
        >
          <p>
            Open pairing on both computers. The entire safety code below must
            match on both screens.
          </p>
          <code className="safety-code">{pair.safety_code}</code>
          <div className="pair-identity">
            <OsIcon os={pair.device.platform} />
            <span>{pair.device.name}</span>
            <small>{pair.address}</small>
          </div>
          <label className="confirm-check">
            <input
              type="checkbox"
              checked={matched}
              onChange={(e) => setMatched(e.target.checked)}
            />
            I compared the codes on both computers and they match.
          </label>
          <p className="footnote">
            Pairing permits input, clipboard, and file sharing. You can unpair
            this computer at any time.
          </p>
          <div className="dialog-actions">
            <button className="button secondary" onClick={() => setPair(null)}>
              Cancel
            </button>
            <button
              className="button primary"
              disabled={!matched}
              onClick={() =>
                void run(async () => {
                  await call("approve_peer", {
                    id: pair.device.id,
                    publicKey: pair.public_key,
                  });
                  setPair(null);
                }, "Computer approved. Approve on the other computer too.")
              }
            >
              <ShieldCheck size={16} />
              Trust this computer
            </button>
          </div>
        </Dialog>
      )}
      {permissionPeer && (
        <Dialog
          title={`Sharing with ${permissionPeer.device.name}`}
          close={() => setPermissionPeer(null)}
        >
          <p>Choose what this paired computer may share with you.</p>
          {(
            [
              ["input", "Allow input", "Share mouse and keyboard control."],
              [
                "clipboard",
                "Allow clipboard",
                "Share enabled clipboard types.",
              ],
              [
                "files",
                "Allow files",
                "Send and receive files with this computer.",
              ],
            ] as const
          ).map(([field, label, description]) => {
            const permissions = data.peer_permissions[
              permissionPeer.device.id
            ] || { input: true, clipboard: true, files: true };
            return (
              <Toggle
                key={field}
                label={label}
                description={description}
                on={permissions[field]}
                change={() =>
                  void run(
                    () =>
                      call("peer_permissions", {
                        id: permissionPeer.device.id,
                        ...permissions,
                        [field]: !permissions[field],
                      }),
                    "Computer permissions saved",
                  )
                }
              />
            );
          })}
          <div className="dialog-actions">
            <button
              className="button primary"
              onClick={() => setPermissionPeer(null)}
            >
              Done
            </button>
          </div>
        </Dialog>
      )}
    </div>
  );
}
function ComputerRow({
  device,
  peer,
  local,
}: {
  device: Device;
  peer?: Peer;
  local?: boolean;
}) {
  return (
    <div className="computer-row">
      <span className={`computer-icon ${local ? "local" : ""}`}>
        <OsIcon os={device.platform} />
      </span>
      <div className="computer-name">
        <strong>{device.name}</strong>
        <small>
          {platform(device.platform)}
          {device.displays.length > 0
            ? ` · ${device.displays.length} display${device.displays.length === 1 ? "" : "s"}`
            : ""}
        </small>
      </div>
      <Badge
        tone={
          local || peer?.connected
            ? "success"
            : peer?.trusted
              ? "neutral"
              : "warning"
        }
      >
        {local
          ? "This computer"
          : peer?.connected
            ? "Connected"
            : peer?.trusted
              ? "Offline"
              : "Ready to pair"}
      </Badge>
      {peer?.latency_ms !== null && peer?.latency_ms !== undefined && (
        <small className="latency">{peer.latency_ms.toFixed(1)} ms RTT</small>
      )}
    </div>
  );
}
function Feature({
  icon: Icon,
  title,
  description,
  on,
}: {
  icon: typeof Keyboard;
  title: string;
  description: string;
  on: boolean;
}) {
  return (
    <div className="feature-row">
      <Icon size={19} />
      <div>
        <strong>{title}</strong>
        <p>{description}</p>
      </div>
      <span className={`feature-check ${on ? "enabled" : ""}`}>
        {on ? <Check size={14} /> : <Pause size={13} />}
      </span>
    </div>
  );
}
function Health({ label, ok }: { label: string; ok: boolean }) {
  return (
    <div className={`health ${ok ? "ok" : ""}`}>
      {ok ? <Check size={18} /> : <LockKeyhole size={18} />}
      <strong>{label}</strong>
      <small>{ok ? "Available" : "Needs attention"}</small>
    </div>
  );
}
function TransferRow({
  t,
  accept,
  cancel,
  copy,
}: {
  t: Transfer;
  accept: (v: boolean) => void;
  cancel: () => void;
  copy: () => void;
}) {
  return (
    <div className="transfer-row">
      <span className="computer-icon">
        <File size={20} />
      </span>
      <div className="transfer-info">
        <strong>{t.name}</strong>
        <small>
          {t.direction === "incoming" ? "Receiving" : "Sending"} ·{" "}
          {bytes(t.bytes)} / {bytes(t.size)} · {t.state.replaceAll("_", " ")}
        </small>
        {t.state === "transferring" && (
          <progress
            value={t.bytes}
            max={Math.max(1, t.size)}
            aria-label={`${t.name} transfer progress`}
          />
        )}
        <p className="device-error">{t.error}</p>
        {t.saved_path && <small>Saved to {t.saved_path}</small>}
        {native &&
          t.direction === "incoming" &&
          t.state === "complete" &&
          t.saved_path && (
            <button
              className="text-button"
              onClick={copy}
              aria-label={`Copy received ${t.name}`}
            >
              Copy file for pasting
            </button>
          )}
      </div>
      {t.state === "offered" ? (
        <div className="transfer-buttons">
          <button className="button secondary" onClick={() => accept(false)}>
            Decline
          </button>
          <button className="button primary" onClick={() => accept(true)}>
            Accept
          </button>
        </div>
      ) : ["transferring", "hashing", "awaiting_acceptance"].includes(
          t.state,
        ) ? (
        <button
          className="icon-button"
          aria-label={`Cancel ${t.name}`}
          onClick={cancel}
        >
          <X size={17} />
        </button>
      ) : t.state === "complete" ? (
        <Badge tone="success">
          <Check size={13} />
          Verified
        </Badge>
      ) : (
        <Badge tone="warning">{t.state}</Badge>
      )}
    </div>
  );
}
function Dialog({
  title,
  close,
  children,
}: {
  title: string;
  close: () => void;
  children: React.ReactNode;
}) {
  const ref = useRef<HTMLDialogElement>(null),
    titleId = useId();
  useEffect(() => {
    ref.current?.showModal();
  }, []);
  return (
    <dialog
      aria-labelledby={titleId}
      ref={ref}
      className="dialog"
      onCancel={close}
      onClick={(e) => {
        if (e.target === ref.current) {
          const r = ref.current.getBoundingClientRect();
          if (
            e.clientX < r.left ||
            e.clientX > r.right ||
            e.clientY < r.top ||
            e.clientY > r.bottom
          )
            close();
        }
      }}
    >
      <div className="section-heading">
        <h2 id={titleId}>{title}</h2>
        <button
          className="icon-button"
          aria-label="Close dialog"
          onClick={close}
        >
          <X size={20} />
        </button>
      </div>
      {children}
    </dialog>
  );
}
function Preferences({
  settings: s,
  change,
  chooseFolder,
}: {
  settings: Settings;
  change: (p: Partial<Settings>) => void;
  chooseFolder: () => Promise<void>;
}) {
  const [name, setName] = useState(s.device_name);
  useEffect(() => setName(s.device_name), [s.device_name]);
  return (
    <div className="preferences-layout">
      <section className="panel">
        <h2>Sharing</h2>
        <Toggle
          label="Text clipboard"
          description="Copy text on one computer and paste on another."
          on={s.clipboard_text}
          change={() => change({ clipboard_text: !s.clipboard_text })}
        />
        <Toggle
          label="Formatted clipboard"
          description="Share HTML formatting when the source app provides it."
          on={s.clipboard_html}
          change={() => change({ clipboard_html: !s.clipboard_html })}
        />
        <Toggle
          label="Image clipboard"
          description="Share images up to the clipboard transfer limit."
          on={s.clipboard_images}
          change={() => change({ clipboard_images: !s.clipboard_images })}
        />
        <Toggle
          label="File clipboard"
          description="Copy regular files in Finder or Explorer, accept them on the other computer, then paste. Enable on both computers. Newer clipboard contents are preserved; zip folders first."
          on={s.clipboard_files}
          change={() => change({ clipboard_files: !s.clipboard_files })}
        />
        <Toggle
          label="Translate Command and Control"
          description="Use familiar shortcuts when moving between Mac and Windows."
          on={s.map_modifiers}
          change={() => change({ map_modifiers: !s.map_modifiers })}
        />
      </section>
      <section className="panel">
        <h2>Cursor behavior</h2>
        <Toggle
          label="Lock cursor to the current screen"
          description="Keep edges from switching computers while you work."
          on={s.cursor_locked}
          change={() => change({ cursor_locked: !s.cursor_locked })}
        />
        <Toggle
          label="Game mode"
          description="Disable automatic edge hops. Raw relative input is not supported yet."
          on={s.game_mode}
          change={() => change({ game_mode: !s.game_mode })}
        />
        <div className="setting-row">
          <div>
            <label htmlFor="sensitivity">
              <strong>Remote pointer sensitivity</strong>
            </label>
            <p>Adjust the distance your cursor moves on the other computer.</p>
          </div>
          <div className="range-control">
            <input
              id="sensitivity"
              type="range"
              min="0.1"
              max="4"
              step="0.1"
              value={s.sensitivity}
              onChange={(e) => change({ sensitivity: Number(e.target.value) })}
            />
            <span>{s.sensitivity.toFixed(1)}×</span>
          </div>
        </div>
        <div className="setting-row">
          <div>
            <label htmlFor="edge-delay">
              <strong>Edge delay</strong>
            </label>
            <p>Wait briefly at an edge to reduce accidental switches.</p>
          </div>
          <select
            id="edge-delay"
            value={s.edge_delay_ms}
            onChange={(e) => change({ edge_delay_ms: Number(e.target.value) })}
          >
            {[0, 80, 150, 300, 500].map((n) => (
              <option key={n} value={n}>
                {n} ms
              </option>
            ))}
          </select>
        </div>
      </section>
      <section className="panel">
        <h2>Receiving files</h2>
        <Toggle
          label="Automatically accept files"
          description="Paired computers can save files without asking. Leave off for shared networks."
          on={s.auto_accept_files}
          change={() => change({ auto_accept_files: !s.auto_accept_files })}
        />
        <div className="setting-row">
          <div>
            <strong>Receiving folder</strong>
            <p className="folder-path">{s.receive_directory}</p>
          </div>
          <button
            className="button secondary"
            onClick={() => void chooseFolder()}
          >
            Choose folder
          </button>
        </div>
        <div className="setting-row">
          <div>
            <label htmlFor="file-limit">
              <strong>Per-file size limit</strong>
            </label>
            <p>Protect the receiving computer’s storage.</p>
          </div>
          <select
            id="file-limit"
            value={s.max_file_bytes}
            onChange={(e) => change({ max_file_bytes: Number(e.target.value) })}
          >
            {[1, 5, 10, 50, 100].map((n) => (
              <option key={n} value={n * 1073741824}>
                {n} GB
              </option>
            ))}
          </select>
        </div>
      </section>
      <section className="panel">
        <h2>This computer</h2>
        <label className="field-label" htmlFor="device-name">
          Computer name
        </label>
        <div className="inline-form">
          <input
            id="device-name"
            maxLength={64}
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
          <button
            className="button secondary"
            disabled={!name.trim() || name === s.device_name}
            onClick={() => change({ device_name: name.trim() })}
          >
            Save name
          </button>
        </div>
        <p className="footnote">
          Closing the window keeps the app in the tray. Use Quit from the tray
          menu to stop sharing.
        </p>
      </section>
    </div>
  );
}
function Arrangement({
  data,
  save,
  add,
  action,
}: {
  data: Snapshot;
  save: (l: LayoutDisplay[]) => Promise<boolean>;
  add: () => void;
  action: (
    command: string,
    args: Record<string, unknown>,
    message: string,
  ) => Promise<boolean>;
}) {
  const [draft, setDraft] = useState(data.layout),
    [dirty, setDirty] = useState(false),
    [selected, setSelected] = useState(""),
    [saving, setSaving] = useState(false),
    [profile, setProfile] = useState(""),
    [profileDialog, setProfileDialog] = useState(false),
    [profileName, setProfileName] = useState("");
  const canvas = useRef<HTMLDivElement>(null),
    [width, setWidth] = useState(760);
  const drag = useRef<{
    key: string;
    x: number;
    y: number;
    left: number;
    top: number;
    scale: number;
  } | null>(null);
  useEffect(() => {
    if (!dirty) setDraft(data.layout);
  }, [data.layout, dirty]);
  useEffect(() => {
    if (!canvas.current) return;
    const o = new ResizeObserver((entries) =>
      setWidth(entries[0].contentRect.width),
    );
    o.observe(canvas.current);
    return () => o.disconnect();
  }, []);
  const visible = data.layout.filter((d) => !d.hidden),
    minX = Math.min(0, ...visible.map((d) => d.rect.x)),
    minY = Math.min(0, ...visible.map((d) => d.rect.y)),
    maxX = Math.max(1920, ...visible.map((d) => d.rect.x + d.rect.width)),
    maxY = Math.max(1080, ...visible.map((d) => d.rect.y + d.rect.height));
  const scale = Math.min(
      (width - 110) / (maxX - minX),
      210 / (maxY - minY),
      0.22,
    ),
    originX = (width - (maxX - minX) * scale) / 2 - minX * scale,
    originY = (285 - (maxY - minY) * scale) / 2 - minY * scale;
  const device = (id: string) =>
    id === data.local.id
      ? data.local
      : data.peers.find((p) => p.device.id === id)?.device;
  const key = (d: LayoutDisplay) => `${d.device_id}/${d.display_id}`;
  const nativeDisplay = (d: LayoutDisplay) =>
    device(d.device_id)?.displays.find((n) => n.id === d.display_id);
  const displayLabel = (d: LayoutDisplay) => {
    const owner = device(d.device_id),
      native = nativeDisplay(d);
    if (!owner || !native) return "Offline display";
    const ordered = [...owner.displays].sort(
      (a, b) =>
        a.bounds.x - b.bounds.x ||
        a.bounds.y - b.bounds.y ||
        a.id.localeCompare(b.id),
    );
    return `Display ${ordered.findIndex((n) => n.id === d.display_id) + 1}${native.primary ? " · Primary" : ""}`;
  };
  const useNativeOrder = () => {
    setDraft((old) => {
      const origins = new Map<
        string,
        { x: number; y: number; nativeX: number; nativeY: number }
      >();
      for (const d of old) {
        const owner = device(d.device_id);
        if (!owner?.displays.length || origins.has(owner.id)) continue;
        const group = old.filter((e) => e.device_id === owner.id);
        origins.set(owner.id, {
          x: Math.min(...group.map((e) => e.rect.x)),
          y: Math.min(...group.map((e) => e.rect.y)),
          nativeX: Math.min(...owner.displays.map((e) => e.bounds.x)),
          nativeY: Math.min(...owner.displays.map((e) => e.bounds.y)),
        });
      }
      return old.map((d) => {
        const origin = origins.get(d.device_id),
          native = nativeDisplay(d);
        return origin && native
          ? {
              ...d,
              rect: {
                x: origin.x + native.bounds.x - origin.nativeX,
                y: origin.y + native.bounds.y - origin.nativeY,
                width: native.bounds.width,
                height: native.bounds.height,
              },
            }
          : d;
      });
    });
    setDirty(true);
  };
  const move = (id: string, x: number, y: number) => {
    setDraft((old) => {
      const target = old.find((d) => key(d) === id);
      if (!target) return old;
      let nx = Math.round(x),
        ny = Math.round(y);
      const tolerance = 80;
      for (const other of old.filter((d) => key(d) !== id && !d.hidden)) {
        for (const snap of [
          other.rect.x - target.rect.width,
          other.rect.x + other.rect.width,
        ])
          if (Math.abs(nx - snap) < tolerance) nx = snap;
        for (const snap of [
          other.rect.y - target.rect.height,
          other.rect.y + other.rect.height,
        ])
          if (Math.abs(ny - snap) < tolerance) ny = snap;
        if (Math.abs(ny - other.rect.y) < tolerance) ny = other.rect.y;
        if (Math.abs(nx - other.rect.x) < tolerance) nx = other.rect.x;
      }
      return old.map((d) =>
        key(d) === id ? { ...d, rect: { ...d.rect, x: nx, y: ny } } : d,
      );
    });
    setDirty(true);
  };
  const submit = async () => {
    setSaving(true);
    const success = await save(draft);
    setSaving(false);
    if (success) setDirty(false);
  };
  return (
    <section className="panel arrangement">
      <div className="arrangement-heading">
        <div>
          <h2>Arrange your displays</h2>
          <p>
            Place screens like they sit on your desk. Touching edges let your
            cursor cross. Save here, then share the arrangement and apply it on
            the other computer for control in both directions.
          </p>
        </div>
        <button className="button secondary" onClick={add}>
          <Plus size={16} />
          Add computer
        </button>
      </div>
      {(data.layout_offers || []).map((offer) => (
        <div className="notice" key={offer.peer_id} role="status">
          <div>
            <strong>
              Arrangement from{" "}
              {data.peers.find((p) => p.device.id === offer.peer_id)?.device
                .name || "paired computer"}
            </strong>
            <p>
              {offer.layout.length} displays. Applying replaces the saved
              positions for these two computers.
            </p>
            <p>
              {offer.layout
                .map(
                  (l) =>
                    `${data.local.id === l.device_id ? data.local.name : data.peers.find((p) => p.device.id === l.device_id)?.device.name || "Computer"}: ${l.rect.x}, ${l.rect.y}${l.hidden ? " (hidden)" : ""}`,
                )
                .join(" · ")}
            </p>
            {dirty && (
              <p>Save or reset your edits before applying this arrangement.</p>
            )}
          </div>
          <button
            className="button secondary"
            disabled={dirty || saving}
            onClick={() =>
              void action(
                "accept_layout",
                { peer: offer.peer_id },
                "Shared arrangement applied",
              )
            }
          >
            Apply arrangement
          </button>
          <button
            className="text-button"
            onClick={() =>
              void action(
                "dismiss_layout",
                { peer: offer.peer_id },
                "Arrangement dismissed",
              )
            }
          >
            Dismiss
          </button>
        </div>
      ))}
      <div className="canvas-toolbar">
        <span>
          <span className="status-dot" />
          Drag to arrange <span className="toolbar-separator">/</span> Select a
          screen and use arrow keys
        </span>
        <div>
          {dirty && <Badge tone="warning">Unsaved changes</Badge>}
          <button
            className="text-button"
            disabled={
              dirty ||
              saving ||
              data.settings.paused ||
              !data.peers.some(
                (p) => p.connected && data.peer_permissions[p.device.id]?.input,
              )
            }
            onClick={() =>
              void action(
                "share_layout",
                {},
                "Arrangement sent. Apply it in Workspace on the other computer.",
              )
            }
          >
            Share arrangement
          </button>
          <button className="text-button" onClick={useNativeOrder}>
            Use OS display order
          </button>
          <button
            className="text-button"
            onClick={() => {
              setDraft(data.layout);
              setDirty(false);
            }}
          >
            Reset
          </button>
          <button
            className="button small primary"
            disabled={!dirty || saving}
            onClick={() => void submit()}
          >
            <Check size={15} />
            {saving ? "Saving…" : "Save layout"}
          </button>
        </div>
      </div>
      <div
        className="display-canvas"
        ref={canvas}
        aria-label="Display arrangement canvas"
      >
        <div className="canvas-hint">
          <MousePointer2 size={14} />
          Your physical desk, mapped
        </div>
        {draft
          .filter((d) => !d.hidden)
          .map((d) => {
            const owner = device(d.device_id);
            const isLocal = d.device_id === data.local.id,
              online =
                isLocal ||
                data.peers.some(
                  (p) => p.device.id === d.device_id && p.connected,
                );
            const id = key(d);
            const chosen = selected === id;
            return (
              <div
                className={`display-tile ${isLocal ? "source" : ""} ${chosen ? "chosen" : ""} ${online ? "" : "offline"}`}
                key={id}
                style={{
                  left: originX + d.rect.x * scale,
                  top: originY + d.rect.y * scale,
                  width: d.rect.width * scale,
                  height: d.rect.height * scale,
                }}
              >
                <button
                  className="screen-drag"
                  aria-label={`Arrange ${owner?.name || "Offline computer"} display${(owner?.displays.length || 0) > 1 ? ` ${displayLabel(d)}` : ""}`}
                  onFocus={() => setSelected(id)}
                  onPointerDown={(e) => {
                    if (e.button !== 0) return;
                    e.currentTarget.setPointerCapture(e.pointerId);
                    drag.current = {
                      key: id,
                      x: e.clientX,
                      y: e.clientY,
                      left: d.rect.x,
                      top: d.rect.y,
                      scale,
                    };
                    setSelected(id);
                  }}
                  onPointerMove={(e) => {
                    const p = drag.current;
                    if (p && p.key === id)
                      move(
                        id,
                        p.left + (e.clientX - p.x) / p.scale,
                        p.top + (e.clientY - p.y) / p.scale,
                      );
                  }}
                  onPointerUp={() => {
                    drag.current = null;
                  }}
                  onPointerCancel={() => {
                    drag.current = null;
                  }}
                  onKeyDown={(e) => {
                    const step = e.shiftKey ? 400 : 80;
                    if (
                      [
                        "ArrowLeft",
                        "ArrowRight",
                        "ArrowUp",
                        "ArrowDown",
                      ].includes(e.key)
                    ) {
                      e.preventDefault();
                      move(
                        id,
                        d.rect.x +
                          (e.key === "ArrowLeft"
                            ? -step
                            : e.key === "ArrowRight"
                              ? step
                              : 0),
                        d.rect.y +
                          (e.key === "ArrowUp"
                            ? -step
                            : e.key === "ArrowDown"
                              ? step
                              : 0),
                      );
                    }
                  }}
                >
                  <span className="screen-top">
                    <OsIcon os={owner?.platform || ""} />
                    <span>
                      {isLocal
                        ? "This computer"
                        : online
                          ? "Connected"
                          : "Offline"}
                    </span>
                    <span className={`status-dot ${online ? "" : "gray"}`} />
                  </span>
                  <strong>{owner?.name || "Offline computer"}</strong>
                  <small>{displayLabel(d)}</small>
                  <small>
                    {d.rect.width} × {d.rect.height}
                  </small>
                  {d.device_id === data.active_device && (
                    <span className="cursor-indicator">
                      <MousePointer2 size={21} fill="currentColor" />
                      <span>Your input</span>
                    </span>
                  )}
                </button>
                <div className="monitor-base" />
                <div className="display-caption">
                  {platform(owner?.platform || "")}
                  <button
                    className="icon-button"
                    aria-label={`Hide ${owner?.name || "offline computer"} display`}
                    onClick={() => {
                      setDraft((old) =>
                        old.map((item) =>
                          key(item) === id ? { ...item, hidden: true } : item,
                        ),
                      );
                      setDirty(true);
                    }}
                  >
                    <EyeOff size={13} />
                  </button>
                </div>
              </div>
            );
          })}
        {!draft.filter((d) => !d.hidden).length && (
          <div className="empty-state">
            <Monitor size={25} />
            <p>Show a display below to arrange your workspace.</p>
          </div>
        )}
      </div>
      <div className="canvas-footer">
        <span>
          <ShieldCheck size={15} />
          Only paired computers receive input.
        </span>
        <span>
          <MousePointer2 size={15} />
          Edges switch automatically
        </span>
      </div>
      {selected &&
        draft.some((d) => key(d) === selected) &&
        (() => {
          const d = draft.find((d) => key(d) === selected)!;
          const binding = data.hop_keys.find(
            (b) => b.device_id === d.device_id && b.display_id === d.display_id,
          );
          const digit = binding?.keys.find((k) => k >= 30 && k <= 38) || 0;
          return (
            <div className="display-inspector">
              <span>
                {device(d.device_id)?.name} · {displayLabel(d)}
              </span>
              <label htmlFor="hop-shortcut">Hop shortcut</label>
              <select
                id="hop-shortcut"
                value={digit}
                onChange={(e) => {
                  const digit = Number(e.target.value);
                  void action(
                    "set_hop_key",
                    {
                      deviceId: d.device_id,
                      displayId: d.display_id,
                      keys: digit ? [224, 226, digit] : [],
                    },
                    "Display shortcut saved",
                  );
                }}
              >
                <option value={0}>No shortcut</option>
                {Array.from({ length: 9 }, (_, i) => (
                  <option key={i} value={i + 30}>
                    Ctrl + Alt + {i + 1}
                  </option>
                ))}
              </select>
              <button
                className="button small secondary"
                disabled={d.hidden || data.settings.paused}
                onClick={() =>
                  void action(
                    "jump_to_display",
                    { deviceId: d.device_id, displayId: d.display_id },
                    "Input moved to the display",
                  )
                }
              >
                Jump here
              </button>
            </div>
          );
        })()}
      <div className="profile-bar">
        <label htmlFor="layout-profile">Layout profile</label>
        <select
          id="layout-profile"
          value={profile}
          onChange={async (e) => {
            const name = e.target.value;
            if (
              name &&
              (await action("load_profile", { name }, "Layout profile applied"))
            ) {
              setDirty(false);
              setProfile(name);
            } else setProfile("");
          }}
        >
          <option value="">Current arrangement</option>
          {data.profiles.map((name) => (
            <option key={name} value={name}>
              {name}
            </option>
          ))}
        </select>
        <button className="text-button" onClick={() => setProfileDialog(true)}>
          Save as profile
        </button>
        {profile && (
          <button
            className="icon-button"
            aria-label={`Delete ${profile} profile`}
            onClick={async () => {
              if (
                await action(
                  "delete_profile",
                  { name: profile },
                  "Layout profile deleted",
                )
              )
                setProfile("");
            }}
          >
            <X size={14} />
          </button>
        )}
      </div>
      {profileDialog && (
        <Dialog
          title="Save layout profile"
          close={() => setProfileDialog(false)}
        >
          <p>Keep an arrangement for a different desk or set of computers.</p>
          <label className="field-label" htmlFor="profile-name">
            Profile name
          </label>
          <input
            id="profile-name"
            maxLength={48}
            value={profileName}
            onChange={(e) => setProfileName(e.target.value)}
            placeholder="Home desk"
          />
          <div className="dialog-actions">
            <button
              className="button secondary"
              onClick={() => setProfileDialog(false)}
            >
              Cancel
            </button>
            <button
              className="button primary"
              disabled={!profileName.trim()}
              onClick={async () => {
                if (dirty && !(await save(draft))) return;
                if (
                  await action(
                    "save_profile",
                    { name: profileName.trim() },
                    "Layout profile saved",
                  )
                ) {
                  setDirty(false);
                  setProfile(profileName.trim());
                  setProfileDialog(false);
                  setProfileName("");
                }
              }}
            >
              Save profile
            </button>
          </div>
        </Dialog>
      )}
      {draft.some((d) => d.hidden) && (
        <div className="hidden-displays">
          <span>Hidden displays</span>
          {draft
            .filter((d) => d.hidden)
            .map((d) => (
              <button
                className="button small secondary"
                key={key(d)}
                onClick={() => {
                  setDraft((old) =>
                    old.map((item) =>
                      key(item) === key(d) ? { ...item, hidden: false } : item,
                    ),
                  );
                  setDirty(true);
                }}
              >
                <Eye size={13} />
                Show {device(d.device_id)?.name || "offline computer"}
              </button>
            ))}
        </div>
      )}
    </section>
  );
}
