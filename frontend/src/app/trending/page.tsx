"use client";
import { useEffect, useState } from "react";
import { Flame } from "lucide-react";
import { apiFetch } from "@/lib/api";
import VideoGrid from "@/components/VideoGrid";
import EmptyState from "@/components/EmptyState";
import ErrorState from "@/components/ErrorState";
import { VideoGridSkeleton } from "@/components/Skeletons";
import type { Video } from "@/lib/types";

export default function TrendingPage() {
  const [videos, setVideos] = useState<Video[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await apiFetch<Video[]>("/api/videos?sort=trending&limit=20");
      setVideos(data);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load trending");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div>
      <h1 className="mb-6 text-2xl font-semibold">Trending</h1>
      {loading ? (
        <VideoGridSkeleton count={10} />
      ) : error ? (
        <ErrorState title="Can't connect" message={error} onRetry={load} />
      ) : videos.length === 0 ? (
        <EmptyState
          icon={<Flame className="size-8 text-muted-foreground" />}
          title="Nothing trending yet"
          subtitle="Trending videos will appear here."
        />
      ) : (
        <VideoGrid videos={videos} />
      )}
    </div>
  );
}
