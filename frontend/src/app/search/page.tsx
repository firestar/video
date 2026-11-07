"use client";
import { Suspense, useEffect, useState } from "react";
import { useSearchParams } from "next/navigation";
import { Search } from "lucide-react";
import { apiFetch } from "@/lib/api";
import VideoGrid from "@/components/VideoGrid";
import EmptyState from "@/components/EmptyState";
import ErrorState from "@/components/ErrorState";
import { VideoGridSkeleton } from "@/components/Skeletons";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import type { Channel, Video } from "@/lib/types";

interface SearchData {
  videos: Video[];
  channels: Channel[];
}

function SearchContent() {
  const params = useSearchParams();
  const [q, setQ] = useState("");
  const [data, setData] = useState<SearchData | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const query = params.get("q") ?? "";

  const retry = () => {
    setError(null);
    setLoading(true);
    apiFetch<SearchData>(`/api/search?q=${encodeURIComponent(query.trim())}&limit=20`)
      .then(setData)
      .catch((e: Error) => setError(e.message))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    const term = query.trim();
    if (!term) {
      setData(null);
      setError(null);
      return;
    }
    const t = setTimeout(() => {
      setLoading(true);
      setError(null);
      apiFetch<SearchData>(`/api/search?q=${encodeURIComponent(term)}&limit=20`)
        .then(setData)
        .catch((e: Error) => setError(e.message))
        .finally(() => setLoading(false));
    }, 300);
    return () => clearTimeout(t);
  }, [query]);

  useEffect(() => setQ(query), [query]);

  const updateQuery = (v: string) => {
    setQ(v);
    const url = new URL(window.location.href);
    if (v.trim()) url.searchParams.set("q", v);
    else url.searchParams.delete("q");
    window.history.replaceState({}, "", url);
  };

  const empty = !query.trim() || (!data && !loading && !error);
  const noResults =
    !loading && !error && data !== null && data.videos.length === 0 && data.channels.length === 0;

  return (
    <div className="mx-auto max-w-6xl px-4 py-6">
      <div className="mb-6 flex items-center gap-3">
        <span className="text-lg font-semibold">Search</span>
        <input
          type="search"
          value={q}
          onChange={(e) => updateQuery(e.target.value)}
          placeholder="Search"
          className="h-10 w-full max-w-md rounded-md border border-border bg-muted/50 px-3 text-sm outline-none"
        />
      </div>

      {loading ? (
        <VideoGridSkeleton count={6} />
      ) : error ? (
        <ErrorState title="Can't connect" message={error} onRetry={retry} />
      ) : empty ? (
        <EmptyState
          icon={<Search className="size-8 text-muted-foreground" />}
          title="Search for videos"
          subtitle="Type something to search."
        />
      ) : noResults ? (
        <EmptyState
          icon={<Search className="size-8 text-muted-foreground" />}
          title={`No results for "${query.trim()}"`}
          subtitle="Try different keywords."
        />
      ) : data ? (
        <div className="space-y-10">
          {data.videos.length > 0 && (
            <section>
              <h2 className="mb-4 text-lg font-semibold">Videos</h2>
              <VideoGrid videos={data.videos} />
            </section>
          )}
          {data.channels.length > 0 && (
            <section>
              <h2 className="mb-4 text-lg font-semibold">Channels</h2>
              <ul className="divide-y divide-border">
                {data.channels.map((c) => (
                  <li key={c.id}>
                    <a
                      href={`/channel/${c.id}`}
                      className="flex items-center gap-3 py-3 hover:bg-muted"
                    >
                      <Avatar size="sm" className="size-10">
                        {c.avatar_url ? (
                          // eslint-disable-next-line @next/next/no-img-element
                          <img
                            src={c.avatar_url}
                            alt={c.name}
                            className="size-full rounded-full object-cover"
                          />
                        ) : (
                          <AvatarFallback>{c.name.charAt(0).toUpperCase()}</AvatarFallback>
                        )}
                      </Avatar>
                      <div className="min-w-0">
                        <div className="truncate text-sm font-medium">{c.name}</div>
                        {c.handle && (
                          <div className="text-xs text-muted-foreground">@{c.handle}</div>
                        )}
                      </div>
                    </a>
                  </li>
                ))}
              </ul>
            </section>
          )}
        </div>
      ) : null}
    </div>
  );
}

export default function SearchPage() {
  return (
    <Suspense
      fallback={
        <div className="mx-auto max-w-6xl px-4 py-6">
          <VideoGridSkeleton count={6} />
        </div>
      }
    >
      <SearchContent />
    </Suspense>
  );
}
