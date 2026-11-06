import { invoke } from "@tauri-apps/api/core";

export interface CommandMetric {
  command: string;
  count: number;
  p50_ms: number;
  p95_ms: number;
  max_ms: number;
}

export interface MainThreadMetric {
  count: number;
  p50_ms: number;
  p95_ms: number;
  max_ms: number;
}

const MAX_COMMAND_SAMPLES = 512;
const MAX_LAG_SAMPLES = 256;
const LAG_INTERVAL_MS = 250;

const commandSamples = new Map<string, number[]>();
const lagSamples: number[] = [];
let lagTimer: ReturnType<typeof setInterval> | null = null;

function percentile(sorted: number[], fraction: number): number {
  if (sorted.length === 0) return 0;
  const index = Math.min(
    sorted.length - 1,
    Math.max(0, Math.ceil(fraction * sorted.length) - 1),
  );
  return sorted[index];
}

function recordSample(samples: number[], value: number, limit: number) {
  if (samples.length >= limit) samples.shift();
  samples.push(value);
}

export function recordCommandDuration(command: string, milliseconds: number) {
  let samples = commandSamples.get(command);
  if (!samples) {
    samples = [];
    commandSamples.set(command, samples);
  }
  recordSample(samples, milliseconds, MAX_COMMAND_SAMPLES);
}

/** Time an IPC call in the webview, including command dispatch overhead. */
export async function timedInvoke<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  const started = performance.now();
  try {
    return await invoke<T>(command, args);
  } finally {
    recordCommandDuration(command, performance.now() - started);
  }
}

export function commandMetrics(): CommandMetric[] {
  return [...commandSamples.entries()]
    .map(([command, samples]) => {
      const sorted = [...samples].sort((left, right) => left - right);
      return {
        command,
        count: sorted.length,
        p50_ms: percentile(sorted, 0.5),
        p95_ms: percentile(sorted, 0.95),
        max_ms: sorted[sorted.length - 1] ?? 0,
      };
    })
    .sort((left, right) => right.p95_ms - left.p95_ms);
}

export function mainThreadMetrics(): MainThreadMetric {
  const sorted = [...lagSamples].sort((left, right) => left - right);
  return {
    count: sorted.length,
    p50_ms: percentile(sorted, 0.5),
    p95_ms: percentile(sorted, 0.95),
    max_ms: sorted[sorted.length - 1] ?? 0,
  };
}

export function resetPerfMetrics() {
  commandSamples.clear();
  lagSamples.length = 0;
}

/** Sample event-loop delay to catch long synchronous work on the UI thread. */
export function startMainThreadSampler(intervalMs = LAG_INTERVAL_MS) {
  if (typeof window === "undefined" || lagTimer !== null) return;
  let expected = performance.now() + intervalMs;
  lagTimer = setInterval(() => {
    const now = performance.now();
    recordSample(lagSamples, Math.max(0, now - expected), MAX_LAG_SAMPLES);
    expected = now + intervalMs;
  }, intervalMs);
}

export function stopMainThreadSampler() {
  if (lagTimer !== null) {
    clearInterval(lagTimer);
    lagTimer = null;
  }
}

declare global {
  interface Window {
    __fm27Perf?: {
      commands: typeof commandMetrics;
      mainThread: typeof mainThreadMetrics;
      reset: typeof resetPerfMetrics;
    };
  }
}

export function installPerfDiagnostics() {
  if (typeof window === "undefined" || window.__fm27Perf) return;
  startMainThreadSampler();
  window.__fm27Perf = {
    commands: commandMetrics,
    mainThread: mainThreadMetrics,
    reset: resetPerfMetrics,
  };
}
