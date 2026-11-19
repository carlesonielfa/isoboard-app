export type BooleanWorkerSender = (value: boolean) => Promise<boolean>;

type PendingRequest = {
  target: boolean;
  resolve: (enabled: boolean) => void;
  reject: (error: unknown) => void;
};

/** Serialize native boolean state changes while always converging on the latest request. */
export function createSerializedBooleanWorker(send: BooleanWorkerSender) {
  let desired = false;
  let applied: boolean | null = false;
  let revision = 0;
  let failedRevision: number | null = null;
  let inFlight = false;
  let pending: PendingRequest | null = null;

  function settle(): void {
    if (inFlight || !pending) return;
    if (failedRevision !== revision && applied !== desired) return;
    const request = pending;
    pending = null;
    request.resolve(failedRevision !== revision && applied === request.target);
  }

  function pump(): void {
    if (inFlight) return;
    if (applied === desired || failedRevision === revision) {
      settle();
      return;
    }

    const target = desired;
    const requestRevision = revision;
    inFlight = true;
    void send(target)
      .then((accepted) => {
        if (accepted) {
          applied = target;
        }
        if (!accepted && requestRevision === revision) {
          failedRevision = requestRevision;
        }
      })
      .catch((error) => {
        applied = null;
        if (requestRevision === revision) {
          failedRevision = requestRevision;
          if (pending?.target === target) {
            const request = pending;
            pending = null;
            request.reject(error);
          }
        }
      })
      .finally(() => {
        inFlight = false;
        settle();
        pump();
      });
  }

  function request(target: boolean): Promise<boolean> {
    desired = target;
    revision += 1;
    failedRevision = null;
    pending?.resolve(false);
    return new Promise((resolve, reject) => {
      pending = { target, resolve, reject };
      pump();
    });
  }

  return { request };
}
