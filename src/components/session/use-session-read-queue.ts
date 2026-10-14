import { useState } from "react";

type ReadJob = { run: () => Promise<void>; failed: (reason: unknown) => void };

export function createSessionReadQueue() {
  let pending: ReadJob | null = null;
  let running = false;
  const drain = async () => {
    if (running) return;
    running = true;
    try {
      while (pending) {
        const job: ReadJob = pending;
        pending = null;
        try { await job.run(); }
        catch (reason) { job.failed(reason); }
      }
    } finally { running = false; }
  };
  return (run: ReadJob["run"], failed: ReadJob["failed"]) => {
    const job = { run, failed };
    pending = job;
    void drain();
    // Cleanup drops unstarted work; its owner rejects results from active native reads.
    return () => { if (pending === job) pending = null; };
  };
}

export function useSessionReadQueue() {
  const [enqueue] = useState(() => createSessionReadQueue());
  return enqueue;
}
