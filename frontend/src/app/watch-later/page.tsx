"use client";
import { Suspense, useCallback, useEffect, useState } from "react";
import { Clock, X } from "lucide-react";
import { apiFetch, getToken } from "@/lib/api";
import { Button } from "@/components/ui/button";
import EmptyState from "@/components/EmptyState";
import ErrorState from "@/components/ErrorState";
import { ListSkeleton } from "@/components/Skeletons";
import { formatCount, formatDate } from "@/lib/format";
import type { Channel } from "@/lib/types";

interface WatchLaterEntry {
  id: string;
  title: string;
  thumbnailUrl?: string | null;
  viewCount: number;
  durationSecs?: number;
  channel: Channel;
}

function WatchLaterContent() {
  const [entries, setEntries] = useState<WatchLaterEntry[] | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [removing, setRemoving] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await apiFetch<{ videos: WatchLaterEntry[] }>("/api/watch-later");
      setEntries(data.videos);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load watch later");
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

  const remove = async (videoId: string) => {
    setRemoving(videoId);
    try {
      await apiFetch(`/api/watch-later/${videoId}`, { method: "DELETE" });
      setEntries((prev) => (prev ? prev.filter((e) => e.id !== videoId) : prev));
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to remove video");
    } finally {
      setRemoving(null);
    }
  };

  if (loading) {
    return (
      <div className="mx-auto max-w-6xl px-4 py-6">
        <h1 className="mb-6 text-xl font-semibold">Watch Later</h1>
        <ListSkeleton count={6} />
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-6xl px-4 py-6">
      <h1 className="mb-6 text-xl font-semibold">Watch Later</h1>

      {error ? (
        <ErrorState title="Can't connect" message={error} onRetry={load} />
      ) : entries && entries.length === 0 ? (
        <EmptyState
          icon={<Clock className="size-8 text-muted-foreground" />}
          title="No videos saved"
          subtitle="Tap “Watch later” on any video to save it here."
        />
      ) : (
        <ul className="flex flex-col gap-2">
          {entries?.map((entry) => (
            <li
              key={entry.id}
              className="flex items-center gap-4 rounded-lg p-2 hover:bg-muted"
            >
              <a href={`/watch/${entry.id}`} className="flex min-w-0 flex-1 items-center gap-4">
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
                  <div className="truncate text-sm font-medium">{entry.title}</div>
                  <div className="text-xs text-muted-foreground">
                    {entry.channel?.name ?? ""}
                  </div>
                  <div className="text-xs text-muted-foreground/70">
                    {formatCount(entry.viewCount)} views
                    {entry.durationSecs != null
                      ? ` · ${Math.floor(entry.durationSecs / 60)}:${String(entry.durationSecs % 60).padStart(2, "0")}`
                      : ""}
                  </div>
                </div>
              </a>
              <Button
                variant="ghost"
                size="icon"
                className="shrink-0"
                onClick={() => remove(entry.id)}
                disabled={removing === entry.id}
                aria-label="Remove from Watch Later"
              >
                <X className="size-4" />
              </Button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

export default function WatchLaterPage() {
  return (
    <Suspense
      fallback={
        <div className="mx-auto max-w-6xl px-4 py-6">
          <h1 className="mb-6 text-xl font-semibold">Watch Later</h1>
          <ListSkeleton count={6} />
        </div>
      }
    >
      <WatchLaterContent />
    </Suspense>
  );
}
