// Console log store — real log system used by all editor subsystems.

export interface LogEntry {
  level: 'info' | 'warn' | 'error' | 'debug';
  message: string;
  timestamp: string;
}

let logs: LogEntry[] = [];
let listeners: (() => void)[] = [];

export function addLog(level: LogEntry['level'], message: string): void {
  logs.push({
    level,
    message,
    timestamp: new Date().toLocaleTimeString('en-GB', {
      hour12: false,
      fractionalSecondDigits: 3,
    }),
  });
  listeners.forEach((fn) => fn());
}

export function getLogs(): LogEntry[] {
  return logs;
}

export function clearLogs(): void {
  logs = [];
  listeners.forEach((fn) => fn());
}

export function subscribeConsole(fn: () => void): () => void {
  listeners.push(fn);
  return () => {
    listeners = listeners.filter((l) => l !== fn);
  };
}

// Convenience helpers
export function logInfo(msg: string): void { addLog('info', msg); }
export function logWarn(msg: string): void { addLog('warn', msg); }
export function logError(msg: string): void { addLog('error', msg); }
export function logDebug(msg: string): void { addLog('debug', msg); }
