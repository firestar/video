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

type Sort = "relevance" | "date" | "views";
type Type = "all" | "video" | "channel";
type Duration = "any" | "short" | "medium" | "long";
type UploadDate = "any" | "hour" | "day" | "week" | "month" | "year";

interface FilterOption {
  value: string;
  label: string;
}

function FilterSelect({
  label,
  value,
  onChange,
  options,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  options: FilterOption[];
}) {
  return (
    <label className="flex items-center gap-1.5 rounded-md border border-border bg-muted/40 px-2.5 py-1.5 text-xs">
      <span className="text-muted-foreground">{label}</span>
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="cursor-pointer bg-transparent font-medium outline-none"
      >
        {options.map((o) => (
          <option key={o.value} value={o.value}>
            {o.label}
          </option>
        ))}
      </select>
    </label>
  );
}

function SearchContent() {
  const params = useSearchParams();
  const [q, setQ] = useState("");
  const [data, setData] = useState<SearchData | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [sort, setSort] = useState<Sort>("relevance");
  const [type, setType] = useState<Type>("all");
  const [duration, setDuration] = useState<Duration>("any");
  const [uploadDate, setUploadDate] = useState<UploadDate>("any");

  const query = params.get("q") ?? "";

  const buildUrl = () => {
    const p = new URLSearchParams();
    p.set("q", query.trim());
    p.set("limit", "20");
    if (sort !== "relevance") p.set("sort", sort);
    if (type !== "all") p.set("type", type);
    if (duration !== "any") p.set("duration", duration);
    if (uploadDate !== "any") p.set("uploadDate", uploadDate);
    return `/api/search?${p.toString()}`;
  };

  const retry = () => {
    setError(null);
    setLoading(true);
    apiFetch<SearchData>(buildUrl())
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
      apiFetch<SearchData>(buildUrl())
        .then(setData)
        .catch((e: Error) => setError(e.message))
        .finally(() => setLoading(false));
    }, 300);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [query, sort, type, duration, uploadDate]);

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
      <div className="mb-4 flex items-center gap-3">
        <span className="text-lg font-semibold">Search</span>
        <input
          type="search"
          value={q}
          onChange={(e) => updateQuery(e.target.value)}
          placeholder="Search"
          className="h-10 w-full max-w-md rounded-md border border-border bg-muted/50 px-3 text-sm outline-none"
        />
      </div>

      {query.trim() && (
        <div className="mb-6 flex flex-wrap items-center gap-2">
          <FilterSelect
            label="Sort by"
            value={sort}
            onChange={(v) => setSort(v as Sort)}
            options={[
              { value: "relevance", label: "Relevance" },
              { value: "date", label: "Upload date" },
              { value: "views", label: "View count" },
            ]}
          />
          <FilterSelect
            label="Type"
            value={type}
            onChange={(v) => setType(v as Type)}
            options={[
              { value: "all", label: "All" },
              { value: "video", label: "Video" },
              { value: "channel", label: "Channel" },
            ]}
          />
          <FilterSelect
            label="Duration"
            value={duration}
            onChange={(v) => setDuration(v as Duration)}
            options={[
              { value: "any", label: "Any" },
              { value: "short", label: "Under 4 min" },
              { value: "medium", label: "4–20 min" },
              { value: "long", label: "Over 20 min" },
            ]}
          />
          <FilterSelect
            label="Upload date"
            value={uploadDate}
            onChange={(v) => setUploadDate(v as UploadDate)}
            options={[
              { value: "any", label: "Any time" },
              { value: "hour", label: "Last hour" },
              { value: "day", label: "Today" },
              { value: "week", label: "This week" },
              { value: "month", label: "This month" },
              { value: "year", label: "This year" },
            ]}
          />
        </div>
      )}

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
          subtitle="Try different keywords or filters."
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
                        {c.avatarUrl ? (
                          // eslint-disable-next-line @next/next/no-img-element
                          <img
                            src={c.avatarUrl}
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
