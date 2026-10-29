"use client";
import { useCallback, useEffect, useState } from "react";
import { apiFetch } from "@/lib/api";
import { Button } from "@/components/ui/button";
import VideoGrid from "@/components/VideoGrid";
import type { Video } from "@/lib/types";

const PAGE_SIZE = 20;

export default function HomePage() {
  const [videos, setVideos] = useState<Video[]>([]);
  const [offset, setOffset] = useState(0);
  const [loading, setLoading] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchPage = useCallback(async (from: number) => {
    const res = await apiFetch<Video[]>(
      `/api/videos?limit=${PAGE_SIZE}&offset=${from}&sort=trending`
    );
    return res;
  }, []);

  useEffect(() => {
    fetchPage(0)
      .then((rows) => setVideos(rows))
      .catch((e: Error) => setError(e.message))
      .finally(() => setLoading(false));
  }, [fetchPage]);

  const loadMore = async () => {
    setLoadingMore(true);
    try {
      const rows = await fetchPage(offset + PAGE_SIZE);
      setVideos((prev) => [...prev, ...rows]);
      setOffset((prev) => prev + PAGE_SIZE);
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setLoadingMore(false);
    }
  };

  return (
    <div>
      <h1 className="mb-6 text-2xl font-semibold">Home Feed</h1>
      {loading ? (
        <div className="flex items-center justify-center py-24 text-white/50">
          Loading videos…
        </div>
      ) : error ? (
        <div className="rounded-lg border border-red-500/30 bg-red-500/10 p-6 text-sm text-red-300">
          Failed to load videos: {error}
        </div>
      ) : (
        <>
          <VideoGrid videos={videos} />
          {videos.length > 0 && (
            <div className="mt-10 flex justify-center">
              <Button onClick={loadMore} disabled={loadingMore}>
                {loadingMore ? "Loading…" : "Load more"}
              </Button>
            </div>
          )}
        </>
      )}
    </div>
  );
}
