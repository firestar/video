"use client";
import { useEffect, useRef, useState } from "react";
import Link from "next/link";
import { apiFetch } from "@/lib/api";

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
      <div className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/85 to-transparent p-4 text-white">
        <Link href={`/watch/${short.id}`} className="text-lg font-semibold hover:underline">
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

  if (error) return <div className="py-24 text-center text-destructive">{error}</div>;
  if (shorts.length === 0)
    return <div className="py-24 text-center text-muted-foreground">No shorts yet.</div>;

  return (
    <div ref={containerRef} className="h-[100dvh] snap-y snap-mandatory overflow-y-auto">
      {shorts.map((s, i) => (
        <div
          key={s.id}
          data-idx={i}
          ref={(el) => {
            itemRefs.current[i] = el;
          }}
          className="h-full w-full snap-start"
        >
          <ShortCard short={s} active={i === activeIdx} />
        </div>
      ))}
    </div>
  );
}
