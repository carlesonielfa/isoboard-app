export type AppError = {
  id: number;
  message: string;
  key: string;
};

export type AppErrorIdentity = {
  source: string;
  revision?: number;
};

export type AppErrorStore = {
  readonly current: AppError | null;
  show: (message: unknown, identity: AppErrorIdentity) => void;
  dismiss: (id: number) => void;
};

export function errorMessage(error: unknown, fallback = 'Something went wrong.'): string {
  if (typeof error === 'object' && error !== null && 'message' in error) {
    const message = String((error as { message?: unknown }).message ?? '').trim();
    if (message) return message;
  }
  if (error instanceof Error && error.message.trim()) return error.message;
  const message = String(error).trim();
  return message && message !== '[object Object]' ? message : fallback;
}

export function errorIdentity(source: string, revision: number | undefined): AppErrorIdentity {
  return { source, revision };
}

export function errorRevision(error: unknown, fallback?: number): number | undefined {
  if (typeof error === 'object' && error !== null && 'revision' in error) {
    const revision = (error as { revision?: unknown }).revision;
    if (typeof revision === 'number' && Number.isFinite(revision)) return revision;
  }
  return fallback;
}

export type AppErrorRegistry = {
  readonly current: AppError | null;
  show: (message: unknown, identity: AppErrorIdentity) => void;
  dismiss: (id: number) => void;
};

export function createAppErrorRegistry(): AppErrorRegistry {
  let current: AppError | null = null;
  let nextId = 0;
  const seenKeys = new Set<string>();

  function keyFor(message: string, identity: AppErrorIdentity): string {
    return `${identity.source}\u0000${identity.revision ?? ''}\u0000${message}`;
  }

  return {
    get current() {
      return current;
    },
    show(message: unknown, identity: AppErrorIdentity) {
      const normalized = errorMessage(message);
      const key = keyFor(normalized, identity);
      if (seenKeys.has(key)) return;
      seenKeys.add(key);
      current = { id: ++nextId, key, message: normalized };
    },
    dismiss(id: number) {
      if (current?.id === id) current = null;
    },
  };
}
