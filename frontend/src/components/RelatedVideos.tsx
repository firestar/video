"use client";

import { useEffect, useState } from "react";
import { apiFetch } from "@/lib/api";
import { formatCount, formatDuration } from "@/lib/format";
import type { Video } from "@/lib/types";

export default function RelatedVideos({ videoId }: { videoId: string }) {
  const [related, setRelated] = useState<Video[]>([]);
  useEffect(() => {
    if (!videoId) return;
    let cancelled = false;
    apiFetch<Video[]>(`/api/videos/${videoId}/related?limit=10`)
      .then((d) => {
        if (!cancelled) setRelated(Array.isArray(d) ? d : []);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [videoId]);

  if (related.length === 0) return null;

  return (
    <section aria-label="Related videos">
      <h2 className="mb-4 text-base font-semibold">Related videos</h2>
      <div className="flex flex-col gap-3">
        {related.map((v) => (
          <RelatedCard key={v.id} video={v} />
        ))}
      </div>
    </section>
  );
}

function RelatedCard({ video }: { video: Video }) {
  const channel = video.channel;
  return (
    <a
      href={`/watch/${video.id}`}
      className="group flex gap-3 rounded-lg p-2 hover:bg-muted"
      aria-label={video.title}
    >
      <div className="relative aspect-video w-28 shrink-0 overflow-hidden rounded-md bg-muted sm:w-40">
        {video.thumbnailUrl ? (
          // eslint-disable-next-line @next/next/no-img-element
          <img
            src={video.thumbnailUrl}
            alt={video.title}
            loading="lazy"
            className="h-full w-full object-cover"
          />
        ) : (
          <div className="flex h-full w-full items-center justify-center text-muted-foreground/50">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor" aria-hidden>
              <path d="M8 5v14l11-7z" />
            </svg>
          </div>
        )}
        {video.durationSecs != null && (
          <span className="absolute bottom-1 right-1 rounded bg-black/80 px-1 py-0.5 text-[10px] font-medium text-white">
            {formatDuration(video.durationSecs)}
          </span>
        )}
      </div>
      <div className="min-w-0 flex-1">
        <h3 className="line-clamp-2 text-sm font-medium leading-snug">{video.title}</h3>
        <div className="mt-1 text-xs text-muted-foreground">{channel?.name ?? "Unknown"}</div>
        <div className="text-xs text-muted-foreground">
          {formatCount(video.viewCount ?? 0)} views
        </div>
      </div>
    </a>
  );
}
