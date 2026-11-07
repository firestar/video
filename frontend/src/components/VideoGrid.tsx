"use client";
import VideoCard from "@/components/VideoCard";
import EmptyState from "@/components/EmptyState";
import { Video } from "lucide-react";
import type { Video as VideoType } from "@/lib/types";

export default function VideoGrid({
  videos,
  reasons,
}: {
  videos: VideoType[];
  reasons?: Record<string, string>;
}) {
  if (videos.length === 0) {
    return (
      <EmptyState
        icon={<Video className="size-8 text-muted-foreground" />}
        title="No videos yet"
        subtitle="Upload a video to get started."
      />
    );
  }
  return (
    <div className="grid grid-cols-1 gap-x-4 gap-y-8 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5">
      {videos.map((v) => (
        <VideoCard key={v.id} video={v} reason={reasons?.[v.id]} />
      ))}
    </div>
  );
}
