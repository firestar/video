"use client";
import { Suspense, useCallback, useEffect, useRef, useState } from "react";
import { useParams } from "next/navigation";
import { apiFetch, getToken } from "@/lib/api";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import { formatCount, formatDate } from "@/lib/format";
import { useAuth } from "@/lib/auth";
import CommentSection from "@/components/CommentSection";
import { Bell, BellOff, ThumbsUp, ThumbsDown, SlidersHorizontal, Check } from "lucide-react";
import RelatedVideos from "@/components/RelatedVideos";
import { WatchPageSkeleton } from "@/components/Skeletons";
import type { Video } from "@/lib/types";

interface Stream {
  quality: string;
  width?: number;
  height?: number;
  sizeBytes?: number;
  url: string;
}

interface WatchData {
  video: Video;
  channel: Video["channel"];
  streamUrl: string | null;
  streams: Stream[];
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
  const [viewCount, setViewCount] = useState(0);
  const [subscribed, setSubscribed] = useState(false);

  const [subHint, setSubHint] = useState<string | null>(null);

  const [quality, setQuality] = useState<string | null>(null);
  const [qualityMenuOpen, setQualityMenuOpen] = useState(false);
  const pendingTimeRef = useRef<number | null>(null);

  const viewedRef = useRef(false);
  const historySentRef = useRef(false);
  const videoRef = useRef<HTMLVideoElement>(null);

  const recordHistory = useCallback(() => {
    if (historySentRef.current || !videoId || !getToken()) return;
    historySentRef.current = true;
    apiFetch("/api/history", {
      method: "POST",
      body: JSON.stringify({ videoId, positionSecs: 30 }),
    }).catch(() => {
      historySentRef.current = false;
    });
  }, [videoId]);

  useEffect(() => {
    if (!videoId) return;
    setLoading(true);
    setError(null);
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
      historySentRef.current = false;
    };
  }, [videoId]);

  const streams = data?.streams ?? [];
  const activeStream =
    streams.find((s) => s.quality === quality) ?? streams[0] ?? null;
  const currentUrl = activeStream?.url ?? data?.streamUrl ?? null;

  const switchQuality = useCallback((q: string) => {
    setQualityMenuOpen(false);
    setQuality((cur) => {
      if (cur === q) return cur;
      const el = videoRef.current;
      if (el) pendingTimeRef.current = el.currentTime;
      return q;
    });
  }, []);

  useEffect(() => {
    if (!data?.streamUrl) return;
    const el = videoRef.current;
    if (!el) return;
    el.muted = true;
    el.play().catch(() => {});
  }, [data?.streamUrl]);
  const toggleLike = useCallback(async () => {
    const next = !liked;
    const prevCount = likeCount;
    setLiked(next);
    setDisliked(false);
    setLikeCount(next ? prevCount + 1 : prevCount - 1);
    if (!getToken()) return;
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
    if (!getToken() || !liked) return;
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
    return <WatchPageSkeleton />;
  }

  if (error || !data) {
    const is404 = error?.includes("404");
    return (
      <div className="flex flex-col items-center gap-4 py-24 text-center">
        <div className="flex size-16 items-center justify-center rounded-full bg-muted">
          <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="text-muted-foreground">
            <circle cx="12" cy="12" r="10" />
            <path d="M12 8v4" />
            <path d="M12 16h.01" />
          </svg>
        </div>
        <div>
          <h1 className="text-xl font-semibold">
            {is404 ? "Video not found" : "Something went wrong"}
          </h1>
          <p className="mt-1 text-sm text-muted-foreground">
            {error ?? "This video may have been removed."}
          </p>
        </div>
        <Button
          variant="outline"
          onClick={() => {
            window.location.href = "/";
          }}
        >
          Go home
        </Button>
      </div>
    );
  }

  const { video, channel } = data;

  return (
    <div className="mx-auto w-full max-w-[1750px] flex-1 px-4 py-4 lg:px-8">
      <div className="flex flex-col gap-6 lg:flex-row lg:gap-8">
        <div className="min-w-0 flex-1">
           {currentUrl ? (
            <div className="relative">
              <video
                key={videoId}
                src={currentUrl}
                ref={videoRef}
                autoPlay
                muted
                controls
                playsInline
                preload="auto"
                onLoadedMetadata={(e) => {
                  if (pendingTimeRef.current != null) {
                    e.currentTarget.currentTime = pendingTimeRef.current;
                    pendingTimeRef.current = null;
                  }
                }}
                onTimeUpdate={(e) => {
                  if (e.currentTarget.currentTime > 30) recordHistory();
                }}
                className="aspect-video w-full rounded-lg bg-black"
              />
              {streams.length > 1 && (
                <div className="absolute right-3 top-3 z-10">
                  <button
                    onClick={() => setQualityMenuOpen((o) => !o)}
                    className="flex items-center gap-1.5 rounded-md bg-black/70 px-2.5 py-1.5 text-xs font-medium text-white backdrop-blur transition-colors hover:bg-black/90"
                  >
                    <SlidersHorizontal className="size-3.5" />
                    {activeStream?.quality ?? "Auto"}
                  </button>
                  {qualityMenuOpen && (
                    <div className="absolute right-0 mt-1 w-36 overflow-hidden rounded-md bg-black/90 py-1 text-xs text-white shadow-lg backdrop-blur">
                      <div className="px-3 py-1.5 text-[10px] uppercase tracking-wide text-white/50">
                        Quality
                      </div>
                      {streams.map((s) => (
                        <button
                          key={s.quality}
                          onClick={() => switchQuality(s.quality)}
                          className={`flex w-full items-center justify-between px-3 py-1.5 hover:bg-white/10 ${
                            s.quality === activeStream?.quality
                              ? "font-semibold"
                              : "text-white/80"
                          }`}
                        >
                          <span>{s.quality}</span>
                          {s.quality === activeStream?.quality && (
                            <Check className="size-3.5" />
                          )}
                        </button>
                      ))}
                    </div>
                  )}
                </div>
              )}
            </div>
          ) : (
             <div className="flex aspect-video w-full items-center justify-center rounded-lg bg-black/60 text-sm text-white/60">
               {video.status === "ready"
                 ? "Stream URL unavailable"
                 : `Video is ${video.status} — not ready to play yet`}
             </div>
           )}
          <h1 className="mt-4 text-xl font-semibold leading-snug">{video.title}</h1>

          <div className="mt-4 flex flex-wrap items-center justify-between gap-4">
            <div className="flex items-center gap-3">
              <a href={`/channel/${channel?.id ?? ""}`} className="shrink-0">
                <Avatar size="lg" className="size-10">
                  {channel?.avatarUrl ? (
                    // eslint-disable-next-line @next/next/no-img-element
                    <img src={channel.avatarUrl} alt={channel.name} className="size-full rounded-full object-cover" />
                  ) : (
                    <AvatarFallback>{(channel?.name ?? "?").charAt(0).toUpperCase()}</AvatarFallback>
                  )}
                </Avatar>
              </a>
              <div>
                <div className="text-sm font-medium">
                  {channel?.name ?? "Unknown channel"}
                </div>
                <div className="text-xs text-muted-foreground">
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
              {subHint && <span className="text-xs text-destructive">{subHint}</span>}
            </div>

            <div className="flex items-center gap-2">
              <div className="flex items-center rounded-full border border-border">
                <button
                  onClick={toggleLike}
                  aria-pressed={liked}
                  className={`flex h-9 items-center gap-2 rounded-l-full pl-4 pr-3 text-sm hover:bg-muted ${
                    liked ? "" : "text-muted-foreground"
                  }`}
                >
                  <ThumbsUp className="size-4" />
                  <span className="tabular-nums">{formatCount(likeCount)}</span>
                </button>
                <span className="h-5 w-px bg-border" />
                <button
                  onClick={toggleDislike}
                  aria-pressed={disliked}
                  className={`flex h-9 items-center gap-2 rounded-r-full pl-3 pr-4 text-sm hover:bg-muted ${
                    disliked ? "" : "text-muted-foreground"
                  }`}
                >
                  <ThumbsDown className="size-4" />
                </button>
              </div>
              <div className="ml-2 text-sm text-muted-foreground">
                {formatCount(viewCount)} views · {formatDate(video.publishedAt ?? video.createdAt)}
              </div>
            </div>
          </div>

          {video.description ? (
            <details className="mt-4 rounded-lg bg-muted p-4">
              <summary className="cursor-pointer text-sm font-medium">
                Description
              </summary>
              <p className="mt-3 whitespace-pre-wrap text-sm text-foreground/80">
                {video.description}
              </p>
            </details>
          ) : (
            <div className="mt-4 rounded-lg bg-muted p-4 text-sm text-muted-foreground">
              No description.
            </div>
          )}

          <div className="mt-6 lg:hidden">
            <RelatedVideos videoId={video.id} />
          </div>

          <div className="mt-6">
            <CommentSection videoId={video.id} />
          </div>
        </div>

        <aside className="hidden w-[400px] shrink-0 lg:block xl:w-[440px]">
          <RelatedVideos videoId={video.id} />
        </aside>
      </div>
    </div>
  );
}

export default function WatchPage() {
  return (
    <Suspense fallback={<WatchPageSkeleton />}>
      <WatchContent />
    </Suspense>
  );
}
