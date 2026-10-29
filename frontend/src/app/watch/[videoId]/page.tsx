"use client";
import { Suspense, useCallback, useEffect, useRef, useState } from "react";
import { useParams } from "next/navigation";
import { apiFetch, getToken } from "@/lib/api";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import { formatCount, formatDate } from "@/lib/format";
import { useAuth } from "@/lib/auth";
import { Bell, BellOff, ThumbsUp, ThumbsDown } from "lucide-react";
import type { Video } from "@/lib/types";

interface WatchData {
  video: Video;
  channel: Video["channel"];
  streamUrl: string | null;
}

function WatchContent() {
  const params = useParams<{ videoId: string }>();
  const videoId = params?.videoId ?? "";
  const { user } = useAuth();

  const [data, setData] = useState<WatchData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [liked, setLiked] = useState(false);
  const [disliked, setDisliked] = useState(false);
  const [likeCount, setLikeCount] = useState(0);
  const [subscribed, setSubscribed] = useState(false);
  const [subHint, setSubHint] = useState<string | null>(null);

  const [viewCount, setViewCount] = useState(0);
  const viewedRef = useRef(false);

  useEffect(() => {
    if (!videoId) return;
    setLoading(true);
    apiFetch<WatchData>(`/api/videos/${videoId}`)
      .then((d) => {
        setData(d);
        setLikeCount(d.video.likeCount ?? 0);
        setViewCount(d.video.viewCount ?? 0);
        if (!viewedRef.current && getToken()) {
          viewedRef.current = true;
          apiFetch(`/api/videos/${videoId}/view`, { method: "POST" }).catch(() => {});
        }
      })
      .catch((e: Error) => setError(e.message))
      .finally(() => setLoading(false));
    return () => {
      viewedRef.current = false;
    };
  }, [videoId]);

  const toggleLike = useCallback(async () => {
    const next = !liked;
    const prevCount = likeCount;
    setLiked(next);
    setDisliked(false);
    setLikeCount(next ? prevCount + 1 : prevCount - 1);
    if (!getToken()) return; // stub: visual only without login
    try {
      if (next) {
        await apiFetch(`/api/videos/${videoId}/like`, { method: "POST" });
      } else {
        await apiFetch(`/api/videos/${videoId}/like`, { method: "DELETE" });
      }
    } catch {
      setLiked(!next);
      setDisliked(false);
      setLikeCount(prevCount);
    }
  }, [liked, likeCount, videoId]);

  const toggleDislike = useCallback(async () => {
    const next = !disliked;
    setDisliked(next);
    setLiked(false);
    if (!getToken() || !liked) return; // stub: no dislike API; drop the like if present
    try {
      await apiFetch(`/api/videos/${videoId}/like`, { method: "DELETE" });
      setLikeCount((c) => Math.max(0, c - 1));
    } catch {
      setDisliked(!next);
      setLiked(true);
    }
  }, [disliked, liked, videoId]);

  const toggleSubscribe = useCallback(() => {
    if (!user) {
      setSubscribed(false);
      window.location.href = "/login";
      return;
    }
    setSubscribed((s) => {
      const next = !s;
      if (data?.channel && next) {
        apiFetch("/api/subscribe", {
          method: "POST",
          body: JSON.stringify({ channelId: data.channel.id }),
        })
          .catch((e: Error) => setSubHint(`Subscribe failed: ${e.message}`))
          .finally(() => setSubHint(null));
      } else if (data?.channel && !next) {
        apiFetch("/api/subscribe", {
          method: "DELETE",
          body: JSON.stringify({ channelId: data.channel.id }),
        })
          .catch(() => {})
          .finally(() => setSubHint(null));
      }
      return next;
    });
  }, [user, data]);

  if (loading) {
    return (
      <div className="flex items-center justify-center py-24 text-white/50">
        Loading video…
      </div>
    );
  }

  if (error || !data) {
    return (
      <div className="rounded-lg border border-red-500/30 bg-red-500/10 p-8 text-sm text-red-300">
        {error ?? "Video not found"}
      </div>
    );
  }

  const { video, channel } = data;

  return (
    <div className="mx-auto flex max-w-6xl flex-col gap-6">
      {/* Player */}
      {data.streamUrl ? (
        <video
          key={videoId}
          src={data.streamUrl}
          controls
          playsInline
          className="aspect-video w-full rounded-lg bg-black"
        />
      ) : (
        <div className="flex aspect-video w-full items-center justify-center rounded-lg bg-black/60 text-sm text-white/60">
          {video.status === "ready"
            ? "Stream URL unavailable"
            : `Video is ${video.status} — not ready to play yet`}
        </div>
      )}

      {/* Title */}
      <h1 className="text-xl font-semibold leading-snug text-white">{video.title}</h1>

      {/* Channel row + actions */}
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <a href={`/channel/${channel?.id ?? ""}`} className="shrink-0">
            <Avatar size="lg" className="size-10">
              <AvatarFallback>{(channel?.name ?? "?").charAt(0).toUpperCase()}</AvatarFallback>
            </Avatar>
          </a>
          <div>
            <div className="text-sm font-medium text-white">
              {channel?.name ?? "Unknown channel"}
            </div>
            <div className="text-xs text-white/60">
              {channel?.handle ? `@${channel.handle}` : ""}
            </div>
          </div>
          <Button
            variant={subscribed ? "secondary" : "default"}
            size="sm"
            className="ml-3"
            onClick={toggleSubscribe}
          >
            {subscribed ? <BellOff /> : <Bell />}
            {subscribed ? "Subscribed" : "Subscribe"}
          </Button>
          {subHint && <span className="text-xs text-red-300">{subHint}</span>}
        </div>

        <div className="flex items-center gap-2">
          <div className="flex items-center rounded-full border border-white/15">
            <button
              onClick={toggleLike}
              aria-pressed={liked}
              className={`flex h-9 items-center gap-2 rounded-l-full pl-4 pr-3 text-sm hover:bg-white/10 ${
                liked ? "text-white" : "text-white/80"
              }`}
            >
              <ThumbsUp className="size-4" />
              <span className="tabular-nums">{formatCount(likeCount)}</span>
            </button>
            <span className="h-5 w-px bg-white/15" />
            <button
              onClick={toggleDislike}
              aria-pressed={disliked}
              className={`flex h-9 items-center gap-2 rounded-r-full pl-3 pr-4 text-sm hover:bg-white/10 ${
                disliked ? "text-white" : "text-white/80"
              }`}
            >
              <ThumbsDown className="size-4" />
            </button>
          </div>
          <div className="ml-2 text-sm text-white/60">
            {formatCount(viewCount)} views · {formatDate(video.publishedAt ?? video.createdAt)}
          </div>
        </div>
      </div>

      {/* Description */}
      {video.description ? (
        <details className="rounded-lg bg-white/5 p-4">
          <summary className="cursor-pointer text-sm font-medium text-white">
            {formatCount(viewCount)} views · {formatDate(video.publishedAt ?? video.createdAt)}
          </summary>
          <p className="mt-3 whitespace-pre-wrap text-sm text-white/80">
            {video.description}
          </p>
        </details>
      ) : (
        <div className="rounded-lg bg-white/5 p-4 text-sm text-white/40">
          No description.
        </div>
      )}
    </div>
  );
}

export default function WatchPage() {
  return (
    <Suspense
      fallback={
        <div className="flex items-center justify-center py-24 text-white/50">
          Loading video…
        </div>
      }
    >
      <WatchContent />
    </Suspense>
  );
}
