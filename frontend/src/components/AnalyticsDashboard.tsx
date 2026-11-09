"use client";

import { useEffect, useMemo, useState } from "react";
import { apiFetch } from "@/lib/api";

interface DayCount {
  day: string; // YYYY-MM-DD
  count: number;
}

interface TopVideo {
  id: string;
  title: string;
  thumbnailUrl: string | null;
  viewCount: number;
  likeCount: number;
}

interface Analytics {
  totalViews: number;
  totalSubscribers: number;
  days: number;
  viewsByDay: DayCount[];
  subscribersByDay: DayCount[];
  topVideos: TopVideo[];
}

/** Build a contiguous list of the last `n` day labels (YYYY-MM-DD), oldest first. */
function lastNDays(n: number): string[] {
  const out: string[] = [];
  const now = new Date();
  for (let i = n - 1; i >= 0; i--) {
    const d = new Date(now);
    d.setDate(now.getDate() - i);
    const pad = (x: number) => x.toString().padStart(2, "0");
    out.push(`${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`);
  }
  return out;
}

/** Merge a sparse {day,count} map into a dense array aligned to `days`. */
function densify(days: string[], sparse: DayCount[]): number[] {
  const map = new Map(sparse.map((d) => [d.day, d.count]));
  return days.map((day) => map.get(day) ?? 0);
}

function shortDay(day: string): string {
  // "2026-11-09" -> "11/09"
  const [, m, d] = day.split("-");
  return `${m}/${d}`;
}

function BarChart({
  title,
  days,
  values,
  color,
}: {
  title: string;
  days: string[];
  values: number[];
  color: string;
}) {
  const max = Math.max(1, ...values);
  const total = values.reduce((a, b) => a + b, 0);
  return (
    <div className="rounded-lg border border-border bg-background p-4">
      <div className="mb-3 flex items-baseline justify-between">
        <h3 className="text-sm font-semibold">{title}</h3>
        <span className="text-xs text-muted-foreground">
          {total.toLocaleString()} in {days.length} days
        </span>
      </div>
      <div className="flex h-32 items-end gap-[2px]">
        {values.map((v, i) => {
          const h = v === 0 ? 2 : Math.max(4, Math.round((v / max) * 100));
          return (
            <div
              key={days[i]}
              className="group relative flex-1"
              title={`${shortDay(days[i])}: ${v.toLocaleString()}`}
            >
              <div
                className={`w-full rounded-sm transition-all ${color}`}
                style={{ height: `${h}%` }}
              />
            </div>
          );
        })}
      </div>
      <div className="mt-2 flex justify-between text-[10px] text-muted-foreground">
        <span>{shortDay(days[0])}</span>
        <span>{shortDay(days[Math.floor(days.length / 2)])}</span>
        <span>{shortDay(days[days.length - 1])}</span>
      </div>
    </div>
  );
}

function StatCard({ label, value }: { label: string; value: number }) {
  return (
    <div className="rounded-lg border border-border bg-background p-4">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="mt-1 text-2xl font-bold tabular-nums">{value.toLocaleString()}</div>
    </div>
  );
}

export default function AnalyticsDashboard({ channelId }: { channelId: string }) {
  const [data, setData] = useState<Analytics | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    apiFetch<Analytics>(`/api/channels/${channelId}/analytics`)
      .then((d) => {
        if (!cancelled) setData(d);
      })
      .catch((e: Error) => {
        if (!cancelled) setError(e.message);
      });
    return () => {
      cancelled = true;
    };
  }, [channelId]);

  const days = useMemo(() => lastNDays(data?.days ?? 28), [data?.days]);
  const views = useMemo(
    () => (data ? densify(days, data.viewsByDay) : []),
    [data, days],
  );
  const subs = useMemo(
    () => (data ? densify(days, data.subscribersByDay) : []),
    [data, days],
  );

  if (error) {
    return (
      <div className="mb-6 rounded-lg border border-border bg-muted/30 p-4 text-sm text-muted-foreground">
        Couldn&apos;t load analytics: {error}
      </div>
    );
  }

  if (!data) {
    return (
      <div className="mb-6 flex items-center gap-3 rounded-lg border border-border bg-background p-4 text-sm text-muted-foreground">
        <div className="size-4 animate-spin rounded-full border-2 border-border border-t-foreground" />
        Loading analytics…
      </div>
    );
  }

  return (
    <div className="mb-6 space-y-4">
      <div>
        <h2 className="text-lg font-semibold">Channel analytics</h2>
        <p className="text-xs text-muted-foreground">Last {data.days} days</p>
      </div>

      <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
        <StatCard label="Total views" value={data.totalViews} />
        <StatCard label="Total subscribers" value={data.totalSubscribers} />
        <StatCard label="Videos" value={data.topVideos.length} />
        <StatCard
          label="Avg views / day"
          value={Math.round(views.reduce((a, b) => a + b, 0) / days.length)}
        />
      </div>

      <div className="grid gap-3 lg:grid-cols-2">
        <BarChart title="Views" days={days} values={views} color="bg-red-500/80" />
        <BarChart title="Subscribers" days={days} values={subs} color="bg-emerald-500/80" />
      </div>

      {data.topVideos.length > 0 && (
        <div className="rounded-lg border border-border bg-background p-4">
          <h3 className="mb-3 text-sm font-semibold">Top videos</h3>
          <div className="flex flex-col gap-2">
            {data.topVideos.map((v, i) => (
              <a
                key={v.id}
                href={`/watch/${v.id}`}
                className="flex items-center gap-3 rounded-md p-1.5 transition-colors hover:bg-muted/40"
              >
                <span className="w-4 text-center text-xs text-muted-foreground">{i + 1}</span>
                <div className="size-12 shrink-0 overflow-hidden rounded bg-muted">
                  {v.thumbnailUrl ? (
                    // eslint-disable-next-line @next/next/no-img-element
                    <img src={v.thumbnailUrl} alt={v.title} className="h-full w-full object-cover" />
                  ) : (
                    <div className="h-full w-full" />
                  )}
                </div>
                <div className="min-w-0 flex-1">
                  <div className="truncate text-sm font-medium">{v.title}</div>
                  <div className="text-xs text-muted-foreground">
                    {v.viewCount.toLocaleString()} views · {v.likeCount.toLocaleString()} likes
                  </div>
                </div>
              </a>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
