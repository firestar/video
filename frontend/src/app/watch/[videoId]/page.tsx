"use client";
import { Suspense, useCallback, useEffect, useRef, useState } from "react";
import { useParams, useRouter } from "next/navigation";
import { apiFetch, getToken } from "@/lib/api";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import { formatCount, formatDate, formatDuration } from "@/lib/format";
import { useAuth } from "@/lib/auth";
import CommentSection from "@/components/CommentSection";
import { Bell, BellOff, ThumbsUp, ThumbsDown, SlidersHorizontal, Check, ListVideo, Plus, Share2, Repeat, Gauge, Clock } from "lucide-react";
import RelatedVideos from "@/components/RelatedVideos";
import WatchQueue from "@/components/WatchQueue";
import { useWatchQueue, WatchQueueProvider } from "@/lib/watch-queue";
import { WatchPageSkeleton } from "@/components/Skeletons";
import type { Video, Playlist, Chapter } from "@/lib/types";

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
  hasLiked?: boolean;
  hasDisliked?: boolean;
  inWatchLater?: boolean;
  chapters: Chapter[];
}
const SPEEDS = [0.25, 0.5, 0.75, 1, 1.25, 1.5, 1.75, 2];

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
  const [dislikeCount, setDislikeCount] = useState(0);
  const [viewCount, setViewCount] = useState(0);
  const [subscribed, setSubscribed] = useState(false);
  const [inWatchLater, setInWatchLater] = useState(false);

  const [subHint, setSubHint] = useState<string | null>(null);

  const [playlistMenuOpen, setPlaylistMenuOpen] = useState(false);
  const [playlists, setPlaylists] = useState<Playlist[]>([]);
  const [addedTo, setAddedTo] = useState<Set<string>>(new Set());
  const [playlistBusy, setPlaylistBusy] = useState(false);
  const [copied, setCopied] = useState(false);
  const shareVideo = useCallback(async () => {
    const url = `${window.location.origin}/watch/${videoId}`;
    try {
      await navigator.clipboard.writeText(url);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Fallback: prompt with the URL
      window.prompt("Copy this link:", url);
    }
  }, [videoId]);

  const loadPlaylists = useCallback(async () => {
    if (!getToken()) return;
    try {
      const data = await apiFetch<{ playlists: Playlist[] }>("/api/playlists");
      setPlaylists(data.playlists);
    } catch {
      setPlaylists([]);
    }
  }, []);

  const openPlaylistMenu = useCallback(() => {
    if (!getToken()) {
      window.location.href = "/login";
      return;
    }
    setPlaylistMenuOpen((o) => {
      const next = !o;
      if (next) loadPlaylists();
      return next;
    });
  }, [loadPlaylists]);

  const addToPlaylist = useCallback(
    async (playlistId: string) => {
      if (!videoId) return;
      setPlaylistBusy(true);
      try {
        const res = await apiFetch<{ added: boolean }>(
          `/api/playlists/${playlistId}/items`,
          { method: "POST", body: JSON.stringify({ videoId }) }
        );
        if (res.added) {
          setAddedTo((s) => new Set(s).add(playlistId));
        }
      } catch {
        // ignore; menu stays open
      } finally {
        setPlaylistBusy(false);
      }
    },
    [videoId]
  );
  const [quality, setQuality] = useState<string | null>(null);
  const [qualityMenuOpen, setQualityMenuOpen] = useState(false);
  const [speed, setSpeed] = useState(1);
  const [loopOn, setLoopOn] = useState(false);
  const [speedMenuOpen, setSpeedMenuOpen] = useState(false);
  const [activeChapter, setActiveChapter] = useState(0);
  const pendingTimeRef = useRef<number | null>(null);

  const viewedRef = useRef(false);
  const historySentRef = useRef(false);
  const videoRef = useRef<HTMLVideoElement>(null);
  const playerRef = useRef<HTMLDivElement>(null);
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

  const router = useRouter();
  const { shift, autoplay } = useWatchQueue();
  const handleEnded = useCallback(() => {
    if (!autoplay) return;
    const next = shift();
    if (next) router.push(`/watch/${next.video.id}`);
  }, [shift, router, autoplay]);

  useEffect(() => {
    if (!videoId) return;
    setLoading(true);
    setError(null);
    apiFetch<WatchData>(`/api/videos/${videoId}`)
      .then((d) => {
        setData(d);
        setLikeCount(d.video.likeCount ?? 0);
        setDislikeCount(d.video.dislikeCount ?? 0);
        setLiked(d.hasLiked ?? false);
        setDisliked(d.hasDisliked ?? false);
        setInWatchLater(d.inWatchLater ?? false);
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
  const chapters = data?.chapters ?? [];
  const seekToChapter = useCallback((secs: number) => {
    const el = videoRef.current;
    if (el) el.currentTime = secs;
  }, []);
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
  // Apply playback rate + loop whenever they change or the media reloads.
  useEffect(() => {
    const el = videoRef.current;
    if (!el) return;
    el.playbackRate = speed;
    el.loop = loopOn;
  }, [speed, loopOn, currentUrl, videoId]);

  // YouTube-style keyboard shortcuts (ignored while typing in a field).
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const el = videoRef.current;
      if (!el) return;
      const t = e.target as HTMLElement | null;
      if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable)) return;
      if (e.metaKey || e.ctrlKey || e.altKey) return;
      const k = e.key.length === 1 ? e.key.toLowerCase() : e.key;
      switch (k) {
        case " ":
        case "k":
          e.preventDefault();
          if (el.paused) el.play().catch(() => {});
          else el.pause();
          break;
        case "j":
          el.currentTime -= 10;
          break;
        case "l":
          el.currentTime += 10;
          break;
        case "arrowleft":
          el.currentTime -= 5;
          break;
        case "arrowright":
          el.currentTime += 5;
          break;
        case ",":
          el.currentTime -= 1;
          break;
        case ".":
          el.currentTime += 1;
          break;
        case "m":
          el.muted = !el.muted;
          break;
        case "f":
          if (document.fullscreenElement) document.exitFullscreen().catch(() => {});
          else playerRef.current?.requestFullscreen().catch(() => {});
          break;
        case "arrowup":
          e.preventDefault();
          el.muted = false;
          el.volume = Math.min(1, el.volume + 0.05);
          break;
        case "arrowdown":
          e.preventDefault();
          el.volume = Math.max(0, el.volume - 0.05);
          break;
        default:
          if (/^[0-9]$/.test(k)) {
            const d = el.duration;
            if (d && isFinite(d)) el.currentTime = (d * parseInt(k, 10)) / 10;
          }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
  const toggleLike = useCallback(async () => {
    const next = !liked;
    const prevLike = likeCount;
    const prevDislike = dislikeCount;
    setLiked(next);
    setDisliked(false);
    setLikeCount(next ? prevLike + 1 : Math.max(0, prevLike - 1));
    if (disliked) setDislikeCount(Math.max(0, prevDislike - 1));
    if (!getToken()) {
      setLiked(!next);
      setLikeCount(prevLike);
      setDislikeCount(prevDislike);
      return;
    }
    try {
      if (next) {
        await apiFetch(`/api/videos/${videoId}/like`, { method: "POST" });
      } else {
        await apiFetch(`/api/videos/${videoId}/like`, { method: "DELETE" });
      }
    } catch {
      setLiked(!next);
      setDisliked(disliked);
      setLikeCount(prevLike);
      setDislikeCount(prevDislike);
    }
  }, [liked, disliked, likeCount, dislikeCount, videoId]);

  const toggleDislike = useCallback(async () => {
    const next = !disliked;
    const prevLike = likeCount;
    const prevDislike = dislikeCount;
    setDisliked(next);
    setLiked(false);
    setDislikeCount(next ? prevDislike + 1 : Math.max(0, prevDislike - 1));
    if (liked) setLikeCount(Math.max(0, prevLike - 1));
    if (!getToken()) {
      setDisliked(!next);
      setDislikeCount(prevDislike);
      setLikeCount(prevLike);
      return;
    }
    try {
      if (next) {
        await apiFetch(`/api/videos/${videoId}/dislike`, { method: "POST" });
      } else {
        await apiFetch(`/api/videos/${videoId}/dislike`, { method: "DELETE" });
      }
    } catch {
      setDisliked(!next);
      setLiked(liked);
      setLikeCount(prevLike);
      setDislikeCount(prevDislike);
    }
  }, [disliked, liked, likeCount, dislikeCount, videoId]);

  const toggleWatchLater = useCallback(async () => {
    if (!getToken()) {
      window.location.href = "/login";
      return;
    }
    const next = !inWatchLater;
    setInWatchLater(next);
    try {
      if (next) {
        await apiFetch("/api/watch-later", {
          method: "POST",
          body: JSON.stringify({ videoId }),
        });
      } else {
        await apiFetch(`/api/watch-later/${videoId}`, { method: "DELETE" });
      }
    } catch {
      setInWatchLater(!next);
    }
  }, [inWatchLater, videoId]);

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
             <div className="relative" ref={playerRef}>
               <video
                 key={videoId}
                 src={currentUrl}
                 ref={videoRef}
                 autoPlay
                 muted
                 controls
                  onTimeUpdate={(e) => {
                    const t = e.currentTarget.currentTime;
                    if (t > 30) recordHistory();
                    const chs = data?.chapters ?? [];
                    if (chs.length > 0) {
                      let idx = 0;
                      for (let i = 0; i < chs.length; i++) {
                        if (t >= chs[i].startSecs) idx = i;
                        else break;
                      }
                      setActiveChapter(idx);
                    }
                  }}
                 onEnded={handleEnded}
                 className="aspect-video w-full rounded-lg bg-black"
               />
               <div className="absolute right-3 top-3 z-10 flex items-start gap-2">
                 <button
                   onClick={() => setLoopOn((l) => !l)}
                   aria-pressed={loopOn}
                   title="Loop"
                   className={`flex items-center gap-1.5 rounded-md px-2.5 py-1.5 text-xs font-medium backdrop-blur transition-colors ${
                     loopOn
                       ? "bg-white text-black"
                       : "bg-black/70 text-white hover:bg-black/90"
                   }`}
                 >
                   <Repeat className="size-3.5" />
                 </button>
                 <div className="relative">
                   <button
                     onClick={() => setSpeedMenuOpen((o) => !o)}
                     title="Playback speed"
                     className="flex items-center gap-1.5 rounded-md bg-black/70 px-2.5 py-1.5 text-xs font-medium text-white backdrop-blur transition-colors hover:bg-black/90"
                   >
                     <Gauge className="size-3.5" />
                     {speed === 1 ? "1x" : `${speed}x`}
                   </button>
                   {speedMenuOpen && (
                     <div className="absolute right-0 mt-1 w-32 overflow-hidden rounded-md bg-black/90 py-1 text-xs text-white shadow-lg backdrop-blur">
                       <div className="px-3 py-1.5 text-[10px] uppercase tracking-wide text-white/50">
                         Speed
                       </div>
                       {SPEEDS.map((s) => (
                         <button
                           key={s}
                           onClick={() => {
                             setSpeed(s);
                             setSpeedMenuOpen(false);
                           }}
                           className={`flex w-full items-center justify-between px-3 py-1.5 hover:bg-white/10 ${
                             s === speed ? "font-semibold" : "text-white/80"
                           }`}
                         >
                           <span>{s === 1 ? "Normal" : `${s}x`}</span>
                           {s === speed && <Check className="size-3.5" />}
                         </button>
                       ))}
                     </div>
                   )}
                 </div>
                 {streams.length > 1 && (
                   <div className="relative">
                     <button
                       onClick={() => setQualityMenuOpen((o) => !o)}
                       title="Quality"
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
             </div>
          ) : (
             <div className="flex aspect-video w-full items-center justify-center rounded-lg bg-black/60 text-sm text-white/60">
               {video.status === "ready"
                 ? "Stream URL unavailable"
                 : `Video is ${video.status} — not ready to play yet`}
             </div>
           )}
           {chapters.length > 0 && (
             <div className="mt-3 flex gap-2 overflow-x-auto pb-1">
               {chapters.map((c, i) => (
                 <button
                   key={i}
                   onClick={() => seekToChapter(c.startSecs)}
                   className={`shrink-0 rounded-full px-3 py-1 text-xs font-medium transition-colors ${
                     i === activeChapter
                       ? "bg-white text-black"
                       : "bg-muted text-muted-foreground hover:bg-muted/80"
                   }`}
                 >
                   {formatDuration(c.startSecs)} {c.title}
                 </button>
               ))}
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
                  <span className="tabular-nums">{formatCount(dislikeCount)}</span>
                </button>
              </div>
              <button
                onClick={shareVideo}
                className={`flex h-9 items-center gap-2 rounded-full border border-border pl-4 pr-3 text-sm hover:bg-muted ${
                  copied ? "text-green-500" : "text-muted-foreground"
                }`}
                aria-label="Share video"
              >
                {copied ? <Check className="size-4" /> : <Share2 className="size-4" />}
                {copied ? "Copied!" : "Share"}
              </button>
               <button
                 onClick={toggleWatchLater}
                 aria-pressed={inWatchLater}
                 className={`flex h-9 items-center gap-2 rounded-full border border-border pl-4 pr-3 text-sm hover:bg-muted ${
                   inWatchLater ? "" : "text-muted-foreground"
                 }`}
               >
                 <Clock className="size-4" />
                 {inWatchLater ? "Saved" : "Watch later"}
               </button>
               <div className="relative">
                <button
                  onClick={openPlaylistMenu}
                  className="flex h-9 items-center gap-2 rounded-full border border-border pl-4 pr-3 text-sm text-muted-foreground hover:bg-muted"
                >
                  <ListVideo className="size-4" />
                  Save
                </button>
                {playlistMenuOpen && (
                  <div className="absolute right-0 top-11 z-20 w-64 overflow-hidden rounded-lg border border-border bg-background shadow-lg">
                    <div className="border-b border-border px-3 py-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                      Save to playlist
                    </div>
                    {playlists.length === 0 ? (
                      <div className="px-3 py-3 text-sm text-muted-foreground">
                        No playlists yet.
                      </div>
                    ) : (
                      <ul className="max-h-64 overflow-y-auto py-1">
                        {playlists.map((p) => {
                          const added = addedTo.has(p.id);
                          return (
                            <li key={p.id}>
                              <button
                                onClick={() => addToPlaylist(p.id)}
                                disabled={added || playlistBusy}
                                className="flex w-full items-center gap-2 px-3 py-2 text-left text-sm hover:bg-muted disabled:cursor-default"
                              >
                                <ListVideo className="size-4 shrink-0 text-muted-foreground" />
                                <span className="min-w-0 flex-1 truncate">{p.title}</span>
                                {added ? (
                                  <Check className="size-4 shrink-0 text-foreground" />
                                ) : (
                                  <Plus className="size-4 shrink-0 text-muted-foreground" />
                                )}
                              </button>
                            </li>
                          );
                        })}
                      </ul>
                    )}
                  </div>
                )}
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

          <div className="mt-6 space-y-4 lg:hidden">
            <WatchQueue currentVideoId={video.id} />
            <RelatedVideos videoId={video.id} />
          </div>

          <div className="mt-6">
            <CommentSection videoId={video.id} />
          </div>
        </div>

        <aside className="hidden w-[400px] shrink-0 space-y-4 lg:block xl:w-[440px]">
          <WatchQueue currentVideoId={video.id} />
          <RelatedVideos videoId={video.id} />
        </aside>
      </div>
    </div>
  );
}

export default function WatchPage() {
  return (
    <WatchQueueProvider>
      <Suspense fallback={<WatchPageSkeleton />}>
        <WatchContent />
      </Suspense>
    </WatchQueueProvider>
  );
}
