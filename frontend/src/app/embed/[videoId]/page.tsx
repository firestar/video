"use client";
import { Suspense, useCallback, useEffect, useRef, useState } from "react";
import Link from "next/link";
import { useParams, useSearchParams } from "next/navigation";
import { apiFetch } from "@/lib/api";
import { Captions, Check, ExternalLink, SlidersHorizontal } from "lucide-react";
import type { Subtitle } from "@/lib/types";

interface Stream {
  quality: string;
  width?: number;
  height?: number;
  sizeBytes?: number;
  url: string;
}

interface EmbedData {
  video: {
    id: string;
    title: string;
    status: string;
  };
  channel?: {
    id: string;
    name: string;
    handle?: string | null;
    avatarUrl?: string | null;
  } | null;
  streams: Stream[];
  streamUrl?: string | null;
  subtitles: Subtitle[];
}

/**
 * Embeddable player: rendered inside an <iframe> on third-party sites.
 * Fetches the public video endpoint without requiring login, plays with
 * native controls, and offers the same quality/CC menus as the watch page.
 * CC is OFF by default (YouTube-embed convention); autoplay is muted so
 * browsers allow it. `?t=SS` starts at an offset, `?autoplay=0` disables it.
 */
function EmbedContent() {
  const params = useParams<{ videoId: string }>();
  const videoId = params?.videoId ?? "";
  const searchParams = useSearchParams();

  const [data, setData] = useState<EmbedData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [quality, setQuality] = useState<string | null>(null);
  const [qualityMenuOpen, setQualityMenuOpen] = useState(false);
  const [ccLang, setCcLang] = useState<string | null>(null);
  const [ccOpen, setCcOpen] = useState(false);
  const subtitlesRef = useRef<Subtitle[]>([]);
  const videoRef = useRef<HTMLVideoElement>(null);
  const pendingTimeRef = useRef<number | null>(null);
  const pendingSeekRef = useRef<number | null>(null);
  const autoplayEnabled = searchParams.get("autoplay") !== "0";

  useEffect(() => {
    if (!videoId) return;
    setLoading(true);
    setError(null);
    apiFetch<EmbedData>(`/api/videos/${videoId}`)
      .then((d) => setData(d))
      .catch((e: Error) => setError(e.message))
      .finally(() => setLoading(false));
  }, [videoId]);

  // Poll while the video is still processing so the embed updates once ready.
  useEffect(() => {
    if (!videoId) return;
    if (!data || data.video.status === "ready") return;
    const id = setInterval(async () => {
      try {
        const d = await apiFetch<EmbedData>(`/api/videos/${videoId}`);
        setData(d);
      } catch {
        // keep last known state
      }
    }, 3000);
    return () => clearInterval(id);
  }, [videoId, data?.video.status]);

  // Start offset from ?t= (seconds).
  useEffect(() => {
    const t = Number(searchParams.get("t"));
    if (Number.isFinite(t) && t > 0) pendingSeekRef.current = t;
  }, [searchParams]);

  const streams = data?.streams ?? [];
  const subtitles = data?.subtitles ?? [];
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

  // Autoplay (muted) once the stream URL is known; restore position after a
  // quality switch.
  useEffect(() => {
    if (!currentUrl) return;
    const el = videoRef.current;
    if (!el) return;
    el.muted = true;
    if (pendingTimeRef.current != null) {
      el.currentTime = pendingTimeRef.current;
      pendingTimeRef.current = null;
    }
    if (autoplayEnabled) el.play().catch(() => {});
  }, [currentUrl, autoplayEnabled]);

  // Apply caption track modes (the `default` attribute alone won't re-apply
  // when the selection or the media changes).
  useEffect(() => {
    subtitlesRef.current = subtitles;
    const el = videoRef.current;
    if (!el) return;
    const apply = () => {
      for (const track of Array.from(el.textTracks)) {
        track.mode = ccLang && track.language === ccLang ? "showing" : "hidden";
      }
    };
    apply();
    el.addEventListener("loadedmetadata", apply);
    return () => el.removeEventListener("loadedmetadata", apply);
  }, [ccLang, subtitles, currentUrl, videoId]);

  // Page title (shown by the embedding site's browser tab / iframe title).
  useEffect(() => {
    if (data) document.title = data.video.title;
  }, [data]);

  if (loading) {
    return (
      <div className="flex h-dvh items-center justify-center bg-black text-sm text-white/60">
        Loading…
      </div>
    );
  }

  if (error || !data) {
    return (
      <div className="flex h-dvh flex-col items-center justify-center gap-3 bg-black px-6 text-center text-white">
        <p className="text-sm font-medium">Video not available</p>
        <p className="text-xs text-white/50">
          It may have been removed or made private.
        </p>
        <Link
          href="/"
          className="text-xs text-white/70 underline underline-offset-2 hover:text-white"
        >
          Go to site
        </Link>
      </div>
    );
  }

  const { video, channel } = data;

  return (
    <div className="flex h-dvh flex-col bg-black text-white">
      <header className="flex shrink-0 items-center gap-2 px-3 py-2">
        <Link
          href={`/watch/${videoId}`}
          className="flex min-w-0 flex-1 items-baseline gap-2 text-xs hover:opacity-80"
        >
          <span className="truncate font-medium">{video.title}</span>
          {channel && (
            <span className="shrink-0 text-white/50">{channel.name}</span>
          )}
        </Link>
        <Link
          href={`/watch/${videoId}`}
          title="Watch on site"
          className="shrink-0 text-white/60 hover:text-white"
        >
          <ExternalLink className="size-3.5" />
        </Link>
      </header>
      <div className="relative min-h-0 flex-1">
        {currentUrl ? (
          <video
            key={videoId}
            src={currentUrl}
            ref={videoRef}
            autoPlay={autoplayEnabled}
            muted
            controls
            onLoadedMetadata={(e) => {
              const seek = pendingSeekRef.current;
              if (seek != null) {
                e.currentTarget.currentTime = seek;
                pendingSeekRef.current = null;
              }
            }}
            className="h-full w-full bg-black object-contain"
          >
            {subtitles.map((s) => (
              <track
                key={s.language}
                kind="subtitles"
                srcLang={s.language}
                src={s.url}
                label={s.language}
              />
            ))}
          </video>
        ) : (
          <div className="flex h-full w-full items-center justify-center text-sm text-white/60">
            {video.status === "ready"
              ? "Stream URL unavailable"
              : `Video is ${video.status} — not ready to play yet`}
          </div>
        )}
        <div className="absolute right-3 top-3 z-10 flex items-start gap-2">
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
          {subtitles.length > 0 && (
            <div className="relative">
              <button
                onClick={() => setCcOpen((o) => !o)}
                title="Subtitles"
                className={`flex items-center gap-1.5 rounded-md px-2.5 py-1.5 text-xs font-medium backdrop-blur transition-colors ${
                  ccLang
                    ? "bg-white text-black"
                    : "bg-black/70 text-white hover:bg-black/90"
                }`}
              >
                <Captions className="size-3.5" />
                {ccLang ?? "CC"}
              </button>
              {ccOpen && (
                <div className="absolute right-0 mt-1 w-36 overflow-hidden rounded-md bg-black/90 py-1 text-xs text-white shadow-lg backdrop-blur">
                  <div className="px-3 py-1.5 text-[10px] uppercase tracking-wide text-white/50">
                    Subtitles
                  </div>
                  <button
                    onClick={() => {
                      setCcLang(null);
                      setCcOpen(false);
                    }}
                    className={`flex w-full items-center justify-between px-3 py-1.5 hover:bg-white/10 ${
                      !ccLang ? "font-semibold" : "text-white/80"
                    }`}
                  >
                    <span>Off</span>
                    {!ccLang && <Check className="size-3.5" />}
                  </button>
                  {subtitles.map((s) => (
                    <button
                      key={s.language}
                      onClick={() => {
                        setCcLang(s.language);
                        setCcOpen(false);
                      }}
                      className={`flex w-full items-center justify-between px-3 py-1.5 hover:bg-white/10 ${
                        ccLang === s.language ? "font-semibold" : "text-white/80"
                      }`}
                    >
                      <span>{s.language}</span>
                      {ccLang === s.language && <Check className="size-3.5" />}
                    </button>
                  ))}
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

export default function EmbedPage() {
  return (
    <Suspense
      fallback={
        <div className="flex h-dvh items-center justify-center bg-black text-sm text-white/60">
          Loading…
        </div>
      }
    >
      <EmbedContent />
    </Suspense>
  );
}
