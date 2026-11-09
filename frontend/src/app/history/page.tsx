"use client";
import { Suspense, useCallback, useEffect, useState } from "react";
import { History } from "lucide-react";
import { apiFetch, getToken } from "@/lib/api";
import { Button } from "@/components/ui/button";
import EmptyState from "@/components/EmptyState";
import ErrorState from "@/components/ErrorState";
import { ListSkeleton } from "@/components/Skeletons";
import { formatCount, formatDate } from "@/lib/format";
import type { Channel } from "@/lib/types";

interface HistoryEntry {
  videoId: string;
  watchedUntilSecs: number;
  updatedAt: string;
  title: string;
  thumbnailUrl: string;
  channel: Channel;
  viewCount: number;
}

function HistoryContent() {
  const [entries, setEntries] = useState<HistoryEntry[] | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [clearing, setClearing] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const entries = await apiFetch<HistoryEntry[]>("/api/history?limit=50");
      setEntries(entries);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load history");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (!getToken()) {
      setEntries([]);
      setLoading(false);
      return;
    }
    load();
  }, [load]);

  const clearHistory = async () => {
    setClearing(true);
    try {
      await apiFetch("/api/history", { method: "DELETE" });
      setEntries([]);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to clear history");
    } finally {
      setClearing(false);
    }
  };

  if (loading) {
    return (
      <div className="mx-auto max-w-6xl px-4 py-6">
        <h1 className="mb-6 text-xl font-semibold">Watch History</h1>
        <ListSkeleton count={6} />
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-6xl px-4 py-6">
      <div className="mb-6 flex items-center justify-between">
        <h1 className="text-xl font-semibold">Watch History</h1>
        {entries && entries.length > 0 && (
          <Button variant="outline" size="sm" onClick={clearHistory} disabled={clearing}>
            {clearing ? "Clearing…" : "Clear all"}
          </Button>
        )}
      </div>

      {error ? (
        <ErrorState title="Can't connect" message={error} onRetry={load} />
      ) : entries && entries.length === 0 ? (
        <EmptyState
          icon={<History className="size-8 text-muted-foreground" />}
          title="No watch history"
          subtitle="Watch a video for 30+ seconds to see it here."
        />
      ) : (
        <ul className="flex flex-col gap-2">
          {entries?.map((entry) => (
            <li key={entry.videoId}>
              <a
                href={
                  entry.watchedUntilSecs > 0
                    ? `/watch/${entry.videoId}?t=${entry.watchedUntilSecs}`
                    : `/watch/${entry.videoId}`
                }
                className="flex items-center gap-4 rounded-lg p-2 hover:bg-muted"
              >
                <div className="flex aspect-video h-14 w-24 shrink-0 items-center justify-center overflow-hidden rounded bg-muted">
                  {entry.thumbnailUrl ? (
                    // eslint-disable-next-line @next/next/no-img-element
                    <img
                      src={entry.thumbnailUrl}
                      alt={entry.title}
                      className="h-full w-full object-cover"
                    />
                  ) : (
                    <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" className="text-muted-foreground/50" aria-hidden>
                      <path d="M8 5v14l11-7z" />
                    </svg>
                  )}
                </div>
                <div className="min-w-0 flex-1">
                  <div className="truncate text-sm font-medium">
                    {entry.title ?? "Video unavailable"}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    {entry.channel?.name ?? ""} · {formatDate(entry.updatedAt)}
                  </div>
                  <div className="text-xs text-muted-foreground/70">
                    Watched to {entry.watchedUntilSecs}s
                    {entry.viewCount != null ? ` · ${formatCount(entry.viewCount)} views` : ""}
                  </div>
                </div>
              </a>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

export default function HistoryPage() {
  return (
    <Suspense
      fallback={
        <div className="mx-auto max-w-6xl px-4 py-6">
          <h1 className="mb-6 text-xl font-semibold">Watch History</h1>
          <ListSkeleton count={6} />
        </div>
      }
    >
      <HistoryContent />
    </Suspense>
  );
}
