"use client";

import { useState } from "react";
import Link from "next/link";
import { ListVideo, X, Trash2, ChevronUp, ChevronDown } from "lucide-react";
import { useWatchQueue } from "@/lib/watch-queue";
import { formatDuration } from "@/lib/format";
import type { Video } from "@/lib/types";

/**
 * The "Up next" queue panel. Shows queued videos in play order with
 * remove / reorder / clear controls. The first item is the next to play.
 */
export default function WatchQueue({ currentVideoId }: { currentVideoId: string }) {
  const { queue, removeFromQueue, clear, autoplay, setAutoplay } = useWatchQueue();
  const [open, setOpen] = useState(true);

  if (queue.length === 0) return null;

  return (
    <section aria-label="Watch queue" className="rounded-lg border border-border">
      <div className="flex items-center justify-between border-b border-border px-4 py-3">
        <button
          type="button"
          onClick={() => setOpen((o) => !o)}
          className="flex items-center gap-2 text-sm font-semibold"
        >
          <ListVideo className="size-4" />
          Up next ({queue.length})
          {open ? (
            <ChevronDown className="size-4 text-muted-foreground" />
          ) : (
            <ChevronUp className="size-4 text-muted-foreground" />
          )}
        </button>
        <button
          type="button"
          onClick={clear}
          className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground"
        >
          <Trash2 className="size-3.5" />
          Clear
        </button>
      </div>
      <div className="flex items-center justify-between border-b border-border px-4 py-2.5">
        <span className="text-sm font-medium">Autoplay</span>
        <button
          type="button"
          role="switch"
          aria-checked={autoplay}
          onClick={() => setAutoplay(!autoplay)}
          className={`relative inline-flex h-5 w-9 shrink-0 items-center rounded-full transition-colors ${
            autoplay ? "bg-green-600" : "bg-zinc-600"
          }`}
        >
          <span
            className={`inline-block size-4 transform rounded-full bg-white shadow transition-transform ${
              autoplay ? "translate-x-[18px]" : "translate-x-0.5"
            }`}
          />
        </button>
      </div>

      {open && (
        <ul className="flex flex-col">
          {queue.map((item, i) => (
            <QueueRow
              key={item.video.id}
              index={i}
              video={item.video}
              isNext={i === 0}
              isCurrent={item.video.id === currentVideoId}
              onRemove={() => removeFromQueue(item.video.id)}
            />
          ))}
        </ul>
      )}
    </section>
  );
}

function QueueRow({
  index,
  video,
  isNext,
  isCurrent,
  onRemove,
}: {
  index: number;
  video: Video;
  isNext: boolean;
  isCurrent: boolean;
  onRemove: () => void;
}) {
  return (
    <li className="group flex items-center gap-3 border-b border-border px-4 py-2 last:border-b-0">
      <span className="w-4 shrink-0 text-center text-xs tabular-nums text-muted-foreground">
        {index + 1}
      </span>
      <Link
        href={`/watch/${video.id}`}
        className="relative aspect-video w-24 shrink-0 overflow-hidden rounded bg-muted"
      >
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
            <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor" aria-hidden>
              <path d="M8 5v14l11-7z" />
            </svg>
          </div>
        )}
        {video.durationSecs != null && (
          <span className="absolute bottom-1 right-1 rounded bg-black/80 px-1 text-[10px] font-medium text-white">
            {formatDuration(video.durationSecs)}
          </span>
        )}
      </Link>
      <div className="min-w-0 flex-1">
        <Link
          href={`/watch/${video.id}`}
          className="line-clamp-2 text-sm font-medium leading-snug hover:underline"
        >
          {video.title}
        </Link>
        <div className="mt-0.5 text-xs text-muted-foreground">
          {video.channel?.name ?? "Unknown"}
          {isNext && <span className="ml-2 text-green-500">· next up</span>}
          {isCurrent && <span className="ml-2 text-amber-500">· now playing</span>}
        </div>
      </div>
      <button
        type="button"
        onClick={onRemove}
        className="shrink-0 rounded-full p-1.5 text-muted-foreground opacity-0 hover:bg-muted hover:text-foreground group-hover:opacity-100"
        aria-label="Remove from queue"
      >
        <X className="size-4" />
      </button>
    </li>
  );
}
