"use client";
import VideoCard from "@/components/VideoCard";
import type { Video } from "@/lib/types";

export default function VideoGrid({ videos }: { videos: Video[] }) {
  if (videos.length === 0) {
    return (
      <div className="flex flex-col items-center gap-2 py-24 text-white/50">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="currentColor" aria-hidden>
          <path d="M8 5v14l11-7z" />
        </svg>
        <p className="text-sm">No videos yet.</p>
        <p className="text-xs">Upload a video to get started.</p>
      </div>
    );
  }
  return (
    <div className="grid grid-cols-1 gap-x-4 gap-y-8 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5">
      {videos.map((v) => (
        <VideoCard key={v.id} video={v} />
      ))}
    </div>
  );
}
