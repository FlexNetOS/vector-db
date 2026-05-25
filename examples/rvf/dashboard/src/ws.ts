export interface LiveEvent {
  event_type: string;
  timestamp: number;
  data: Record<string, unknown>;
}

type EventCallback = (event: LiveEvent) => void;

const listeners: EventCallback[] = [];
let socket: WebSocket | null = null;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let reconnectDelay = 1000;
let intentionalClose = false;

const MAX_RECONNECT_DELAY = 30000;
const RECONNECT_BACKOFF = 2;

// Offline development flag. When VITE_MOCK_API=true, we skip the real
// WebSocket and emit a few canned LiveEvents on a short interval so UI views
// that subscribe via onEvent() have something to render.
const MOCK_API = import.meta.env.VITE_MOCK_API === 'true';
let mockTimer: ReturnType<typeof setInterval> | null = null;

function emitMockEvents(): void {
  const now = Date.now();
  const events: LiveEvent[] = [
    { event_type: 'witness.append', timestamp: now, data: { hash: '0x52564deadbeef', epoch: 1 } },
    { event_type: 'coherence.update', timestamp: now + 1, data: { target_id: 'TRAPPIST-1e', value: 0.91 } },
    { event_type: 'pipeline.heartbeat', timestamp: now + 2, data: { uptime: 3600 } },
  ];
  for (const ev of events) {
    for (const cb of listeners) cb(ev);
  }
}

function handleMessage(raw: MessageEvent): void {
  try {
    const event = JSON.parse(raw.data as string) as LiveEvent;
    for (const cb of listeners) {
      cb(event);
    }
  } catch {
    // Ignore malformed messages
  }
}

function scheduleReconnect(): void {
  if (intentionalClose) return;
  if (reconnectTimer) return;

  reconnectTimer = setTimeout(() => {
    reconnectTimer = null;
    openSocket();
  }, reconnectDelay);

  reconnectDelay = Math.min(reconnectDelay * RECONNECT_BACKOFF, MAX_RECONNECT_DELAY);
}

function openSocket(): void {
  if (socket && (socket.readyState === WebSocket.OPEN || socket.readyState === WebSocket.CONNECTING)) {
    return;
  }

  const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
  const url = `${protocol}//${location.host}/ws/live`;

  try {
    socket = new WebSocket(url);
  } catch {
    scheduleReconnect();
    return;
  }

  socket.addEventListener('open', () => {
    reconnectDelay = 1000;
  });

  socket.addEventListener('message', handleMessage);

  socket.addEventListener('close', () => {
    socket = null;
    scheduleReconnect();
  });

  socket.addEventListener('error', () => {
    socket?.close();
  });
}

export function onEvent(callback: EventCallback): () => void {
  listeners.push(callback);
  return () => {
    const idx = listeners.indexOf(callback);
    if (idx >= 0) listeners.splice(idx, 1);
  };
}

export function connect(): void {
  intentionalClose = false;
  reconnectDelay = 1000;
  if (MOCK_API) {
    if (!mockTimer) {
      // Initial burst on next tick so subscribers registered after connect()
      // still observe the first batch.
      setTimeout(emitMockEvents, 100);
      mockTimer = setInterval(emitMockEvents, 5000);
    }
    return;
  }
  openSocket();
}

export function disconnect(): void {
  intentionalClose = true;
  if (mockTimer) {
    clearInterval(mockTimer);
    mockTimer = null;
  }
  if (reconnectTimer) {
    clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }
  if (socket) {
    socket.close();
    socket = null;
  }
}
