"use client";
import { useCallback, useEffect, useRef, useState } from "react";
import Link from "next/link";
import { ThumbsUp, MessageCircle, Share2, Check } from "lucide-react";
import { apiFetch, getToken } from "@/lib/api";

interface ShortItem {
  id: string;
  title: string;
  thumbnailUrl?: string;
  durationSecs?: number;
  viewCount: number;
  likeCount: number;
  channelId: string;
  channelName: string;
  channelAvatarUrl?: string;
  createdAt: string;
}

interface Stream {
  url: string;
  quality: string;
}

function formatViews(n: number): string {
  if (n >= 1e6) return (n / 1e6).toFixed(1) + "M";
  if (n >= 1e3) return (n / 1e3).toFixed(1) + "K";
  return String(n);
}

function ShortCard({ short, active }: { short: ShortItem; active: boolean }) {
  const [src, setSrc] = useState<string | null>(null);
  const [loaded, setLoaded] = useState(false);
  const videoRef = useRef<HTMLVideoElement>(null);
  const [liked, setLiked] = useState(false);
  const [likeCount, setLikeCount] = useState(short.likeCount);
  const [copied, setCopied] = useState(false);

  // Lazily fetch the stream URLs when this card first becomes active.
  useEffect(() => {
    if (!active || loaded) return;
    let cancelled = false;
    (async () => {
      try {
        const v = await apiFetch<{ streams: Stream[] }>(`/api/videos/${short.id}`);
        if (cancelled) return;
        const s = v.streams?.[0];
        if (s) setSrc(s.url);
      } catch {
        /* fall through to loaded */
      }
      setLoaded(true);
    })();
    return () => {
      cancelled = true;
    };
  }, [active, loaded, short.id]);

  // Play while active, pause otherwise.
  useEffect(() => {
    const el = videoRef.current;
    if (!el) return;
    if (active) el.play().catch(() => {});
    else el.pause();
  }, [active, src]);

  const toggleLike = useCallback(async () => {
    const next = !liked;
    const prev = likeCount;
    setLiked(next);
    setLikeCount(next ? prev + 1 : Math.max(0, prev - 1));
    if (!getToken()) {
      setLiked(!next);
      setLikeCount(prev);
      return;
    }
    try {
      await apiFetch(`/api/videos/${short.id}/like`, { method: next ? "POST" : "DELETE" });
    } catch {
      setLiked(!next);
      setLikeCount(prev);
    }
  }, [liked, likeCount, short.id]);

  const share = useCallback(async () => {
    const url = `${window.location.origin}/watch/${short.id}`;
    try {
      await navigator.clipboard.writeText(url);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      window.prompt("Copy this link:", url);
    }
  }, [short.id]);

  return (
    <div className="relative flex h-full w-full items-center justify-center bg-black">
      {src ? (
        <video
          ref={videoRef}
          src={src}
          muted
          loop
          playsInline
          autoPlay
          className="h-full w-full object-contain"
        />
      ) : (
        <div className="flex flex-col items-center gap-3 text-white/70">
          {short.thumbnailUrl ? (
            <img src={short.thumbnailUrl} alt="" className="h-72 w-40 rounded object-cover" />
          ) : (
            <div className="text-5xl">🎬</div>
          )}
          <p className="text-sm">Loading…</p>
        </div>
      )}
      <div className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/85 to-transparent p-4 pr-20 text-white">
        <Link href={`/watch/${short.id}`} data-testid="short-title" className="text-lg font-semibold hover:underline">
          {short.title}
        </Link>
        <div className="mt-1 flex items-center gap-2 text-sm text-white/80">
          {short.channelAvatarUrl ? (
            <img src={short.channelAvatarUrl} alt="" className="h-5 w-5 rounded-full" />
          ) : null}
          <span>{short.channelName}</span>
          <span>· {formatViews(short.viewCount)} views</span>
        </div>
      </div>
      {/* Right action rail — shown on the active card. */}
      {active && (
        <div className="absolute bottom-24 right-3 flex flex-col items-center gap-5">
          <button
            onClick={toggleLike}
            aria-pressed={liked}
            aria-label="Like short"
            className="flex flex-col items-center gap-1 text-white"
          >
            <span
              className={`flex h-11 w-11 items-center justify-center rounded-full transition-colors ${
                liked ? "bg-white text-black" : "bg-white/20 backdrop-blur hover:bg-white/30"
              }`}
            >
              <ThumbsUp className="size-5" />
            </span>
            <span className="text-xs font-medium">{formatViews(likeCount)}</span>
          </button>
          <Link href={`/watch/${short.id}`} className="flex flex-col items-center gap-1 text-white">
            <span className="flex h-11 w-11 items-center justify-center rounded-full bg-white/20 backdrop-blur hover:bg-white/30">
              <MessageCircle className="size-5" />
            </span>
            <span className="text-xs font-medium">Comment</span>
          </Link>
          <button onClick={share} aria-label="Share short" className="flex flex-col items-center gap-1 text-white">
            <span className="flex h-11 w-11 items-center justify-center rounded-full bg-white/20 backdrop-blur hover:bg-white/30">
              {copied ? <Check className="size-5" /> : <Share2 className="size-5" />}
            </span>
            <span className="text-xs font-medium">{copied ? "Copied!" : "Share"}</span>
          </button>
        </div>
      )}
    </div>
  );
}

export default function ShortsPage() {
  const [shorts, setShorts] = useState<ShortItem[]>([]);
  const [activeIdx, setActiveIdx] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const itemRefs = useRef<(HTMLDivElement | null)[]>([]);

  useEffect(() => {
    (async () => {
      try {
        const data = await apiFetch<{ shorts: ShortItem[] }>("/api/shorts?limit=30");
        setShorts(data.shorts);
      } catch (e) {
        setError(e instanceof Error ? e.message : "Failed to load shorts");
      }
    })();
  }, []);

  // Track which short is in view (drives play/pause).
  useEffect(() => {
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            const idx = Number((entry.target as HTMLElement).dataset.idx);
            setActiveIdx(idx);
          }
        }
      },
      { root: containerRef.current, threshold: 0.6 }
    );
    itemRefs.current.forEach((el) => el && observer.observe(el));
    return () => observer.disconnect();
  }, [shorts]);
  // Focus the feed once it's rendered so keyboard navigation works.
  useEffect(() => {
    if (shorts.length > 0) containerRef.current?.focus();
  }, [shorts.length]);

  // Keyboard navigation: move one short at a time (arrows / PageUp / PageDown).
  const handleKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    const el = containerRef.current;
    if (!el) return;
    const step = el.clientHeight;
    if (e.key === "ArrowDown" || e.key === "PageDown") {
      e.preventDefault();
      el.scrollBy({ top: step, behavior: "smooth" });
    } else if (e.key === "ArrowUp" || e.key === "PageUp") {
      e.preventDefault();
      el.scrollBy({ top: -step, behavior: "smooth" });
    }
  };

  const heading = <h1 className="mb-3 shrink-0 text-2xl font-semibold">Shorts</h1>;

  if (error)
    return (
      <div>
        {heading}
        <div className="py-24 text-center text-destructive">{error}</div>
      </div>
    );
  if (shorts.length === 0)
    return (
      <div>
        {heading}
        <div className="py-24 text-center text-muted-foreground">No shorts yet.</div>
      </div>
    );

  return (
    <div>
      {heading}
      <div
        ref={containerRef}
        data-testid="shorts-feed"
        tabIndex={0}
        onKeyDown={handleKeyDown}
        className="h-[calc(100dvh-132px)] sm:h-[calc(100dvh-148px)] snap-y snap-mandatory overflow-y-auto overscroll-contain focus:outline-none"
      >
        {shorts.map((s, i) => (
          <div
            key={s.id}
            data-idx={i}
            data-testid="short-card"
            ref={(el) => {
              itemRefs.current[i] = el;
            }}
            className="h-full w-full snap-start"
          >
            <ShortCard short={s} active={i === activeIdx} />
          </div>
        ))}
      </div>
    </div>
  );
}
