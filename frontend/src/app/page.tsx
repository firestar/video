"use client";
import { useCallback, useEffect, useState } from "react";
import { apiFetch } from "@/lib/api";
import { Button } from "@/components/ui/button";
import VideoGrid from "@/components/VideoGrid";
import { VideoGridSkeleton } from "@/components/Skeletons";
import ErrorState from "@/components/ErrorState";
import type { FeedResponse, Video } from "@/lib/types";

const PAGE_SIZE = 20;

export default function HomePage() {
  const [videos, setVideos] = useState<Video[]>([]);
  const [reasons, setReasons] = useState<Record<string, string>>({});
  const [offset, setOffset] = useState(0);
  const [loading, setLoading] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadInitial = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await apiFetch<FeedResponse>(`/api/home/feed?limit=${PAGE_SIZE}`);
      setVideos(data.videos);
      setReasons(data.reasons);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load videos");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadInitial();
  }, [loadInitial]);

  const loadMore = async () => {
    setLoadingMore(true);
    try {
      const data = await apiFetch<FeedResponse>(`/api/home/feed?limit=${offset + PAGE_SIZE}`);
      setVideos(data.videos);
      setReasons(data.reasons);
      setOffset((prev) => prev + PAGE_SIZE);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load more");
    } finally {
      setLoadingMore(false);
    }
  };

  if (loading) {
    return (
      <div>
        <h1 className="mb-6 text-2xl font-semibold">Home Feed</h1>
        <VideoGridSkeleton count={10} />
      </div>
    );
  }

  if (error) {
    return (
      <div>
        <h1 className="mb-6 text-2xl font-semibold">Home Feed</h1>
        <ErrorState
          title="Can't connect"
          message={error}
          onRetry={loadInitial}
        />
      </div>
    );
  }

  return (
    <div>
      <h1 className="mb-6 text-2xl font-semibold">Home Feed</h1>
      <VideoGrid videos={videos} reasons={reasons} />
      {videos.length > 0 && (
        <div className="mt-10 flex justify-center">
          <Button onClick={loadMore} disabled={loadingMore}>
            {loadingMore ? "Loading…" : "Load more"}
          </Button>
        </div>
      )}
    </div>
  );
}
