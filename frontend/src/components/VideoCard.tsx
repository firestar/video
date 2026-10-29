"use client";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { formatCount, formatDuration } from "@/lib/format";
import type { Video } from "@/lib/types";

export default function VideoCard({ video }: { video: Video }) {
  const channel = video.channel;
  return (
    <a
      href={`/watch/${video.id}`}
      className="group flex flex-col gap-3"
      aria-label={video.title}
    >
      <div className="relative aspect-video w-full overflow-hidden rounded-lg bg-white/5">
        {video.thumbnailUrl ? (
          // eslint-disable-next-line @next/next/no-img-element
          <img
            src={video.thumbnailUrl}
            alt={video.title}
            loading="lazy"
            className="h-full w-full object-cover transition-transform duration-200 group-hover:scale-[1.02]"
          />
        ) : (
          <div className="flex h-full w-full items-center justify-center text-white/30">
            <svg width="40" height="40" viewBox="0 0 24 24" fill="currentColor" aria-hidden>
              <path d="M8 5v14l11-7z" />
            </svg>
          </div>
        )}
        {video.durationSecs != null && (
          <span className="absolute bottom-1.5 right-1.5 rounded bg-black/80 px-1.5 py-0.5 text-xs font-medium tabular-nums text-white">
            {formatDuration(video.durationSecs)}
          </span>
        )}
      </div>
      <div className="flex gap-3">
        <Avatar size="sm" className="mt-0.5 size-9">
          <AvatarFallback>{(channel?.name ?? "?").charAt(0).toUpperCase()}</AvatarFallback>
        </Avatar>
        <div className="min-w-0">
          <h3 className="line-clamp-2 text-sm font-medium leading-snug text-white">
            {video.title}
          </h3>
          <div className="mt-1 text-xs text-white/60">{channel?.name ?? "Unknown channel"}</div>
          <div className="text-xs text-white/60">{formatCount(video.viewCount ?? 0)} views</div>
        </div>
      </div>
    </a>
  );
}
