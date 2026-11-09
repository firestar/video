"use client";

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import type { Video } from "@/lib/types";

/** A queued video: the full Video plus the position it was added at. */
export interface QueueItem {
  video: Video;
  addedAt: number;
}

interface WatchQueueValue {
  /** Queued videos in play order (front = next to play). */
  queue: QueueItem[];
  /** Add a video to the end of the queue. Returns false if already queued. */
  addToQueue: (video: Video) => boolean;
  /** Remove a video from the queue by id. */
  removeFromQueue: (videoId: string) => void;
  /** Remove and return the front item (the next video to play). */
  shift: () => QueueItem | undefined;
  /** Clear the whole queue. */
  clear: () => void;
  /** Whether a given video is already in the queue. */
  isInQueue: (videoId: string) => boolean;
  /** Whether the next queued video auto-plays when the current one ends. */
  autoplay: boolean;
  setAutoplay: (v: boolean) => void;
}

const WatchQueueContext = createContext<WatchQueueValue | null>(null);

const STORAGE_KEY = "yt-clone-watch-queue";
const AUTOPLAY_KEY = "yt-clone-autoplay";

function loadQueue(): QueueItem[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter(
      (x): x is QueueItem =>
        x && typeof x === "object" && x.video && typeof x.video.id === "string"
    );
  } catch {
    return [];
  }
}

export function WatchQueueProvider({ children }: { children: ReactNode }) {
  const [queue, setQueue] = useState<QueueItem[]>([]);
  const [hydrated, setHydrated] = useState(false);
  const [autoplay, setAutoplayState] = useState(true);

  // Hydrate from localStorage once on mount (avoids SSR mismatch).
  useEffect(() => {
    setQueue(loadQueue());
    try {
      const raw = window.localStorage.getItem(AUTOPLAY_KEY);
      if (raw !== null) setAutoplayState(raw === "true");
    } catch {
      // ignore
    }
    setHydrated(true);
  }, []);

  // Persist on change (after hydration).
  useEffect(() => {
    if (!hydrated) return;
    try {
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify(queue));
    } catch {
      // ignore quota / privacy-mode errors
    }
  }, [queue, hydrated]);

  const setAutoplay = useCallback((v: boolean) => {
    setAutoplayState(v);
    try {
      window.localStorage.setItem(AUTOPLAY_KEY, String(v));
    } catch {
      // ignore quota / privacy-mode errors
    }
  }, []);

  const addToQueue = useCallback((video: Video) => {
    if (queue.some((it) => it.video.id === video.id)) return false;
    setQueue((q) =>
      q.some((it) => it.video.id === video.id)
        ? q
        : [...q, { video, addedAt: Date.now() }]
    );
    return true;
  }, [queue]);

  const removeFromQueue = useCallback((videoId: string) => {
    setQueue((q) => q.filter((it) => it.video.id !== videoId));
  }, []);

  const shift = useCallback((): QueueItem | undefined => {
    const next = queue[0];
    if (next) setQueue((q) => q.slice(1));
    return next;
  }, [queue]);

  const clear = useCallback(() => setQueue([]), []);

  const isInQueue = useCallback(
    (videoId: string) => queue.some((it) => it.video.id === videoId),
    [queue]
  );

  const value = useMemo<WatchQueueValue>(
    () => ({ queue, addToQueue, removeFromQueue, shift, clear, isInQueue, autoplay, setAutoplay }),
    [queue, addToQueue, removeFromQueue, shift, clear, isInQueue, autoplay, setAutoplay]
  );

  return (
    <WatchQueueContext.Provider value={value}>{children}</WatchQueueContext.Provider>
  );
}

export function useWatchQueue(): WatchQueueValue {
  const ctx = useContext(WatchQueueContext);
  if (!ctx) {
    throw new Error("useWatchQueue must be used within a WatchQueueProvider");
  }
  return ctx;
}
