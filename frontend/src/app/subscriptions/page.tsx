"use client";
import { Suspense, useEffect, useState } from "react";
import Link from "next/link";
import { apiFetch, getToken } from "@/lib/api";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import type { Channel } from "@/lib/types";

interface SubEntry {
  channel: Channel;
  subscriberCount: number;
}

function SubscriptionsContent() {
  const [items, setItems] = useState<SubEntry[] | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!getToken()) {
      setLoading(false);
      setItems([]);
      return;
    }
    setLoading(true);
    setError(null);
    apiFetch<SubEntry[]>("/api/subscriptions")
      .then(setItems)
      .catch((e: Error) => setError(e.message))
      .finally(() => setLoading(false));
  }, []);

  if (loading) {
    return (
      <div className="flex items-center justify-center py-24 text-white/50">
        Loading subscriptions…
      </div>
    );
  }

  if (error) {
    return (
      <div className="mx-auto max-w-6xl px-4">
        <div className="rounded-lg border border-red-500/30 bg-red-500/10 p-8 text-sm text-red-300">
          {error}
        </div>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-6xl px-4 py-6">
      <h1 className="mb-6 text-xl font-semibold text-white">Subscriptions</h1>

      {items && items.length === 0 ? (
        <div className="flex flex-col items-center gap-2 py-24 text-white/50">
          <svg
            width="48"
            height="48"
            viewBox="0 0 24 24"
            fill="currentColor"
            aria-hidden
          >
            <path d="M21 8h-2V6a2 2 0 0 0-2-2H7a2 2 0 0 0-2 2v2H3a1 1 0 0 0-1 1v9a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V9a1 1 0 0 0-1-1ZM5 6a1 1 0 0 1 1-1h12a1 1 0 0 1 1 1v2H5V6Z" />
          </svg>
          <p className="text-sm">No subscriptions yet</p>
          <p className="text-xs">
            Subscribe to channels to see their videos here.
          </p>
        </div>
      ) : (
        <ul className="flex flex-col gap-2">
          {items?.map((entry) => (
            <li key={entry.channel.id}>
              <Link
                href={`/channel/${entry.channel.id}`}
                className="flex items-center gap-4 rounded-lg p-2 hover:bg-white/5"
              >
                <Avatar size="lg" className="size-10">
                  {entry.channel.avatar_url ? (
                    <AvatarImage
                      src={entry.channel.avatar_url}
                      alt={entry.channel.name}
                    />
                  ) : (
                    <AvatarFallback>
                      {entry.channel.name.charAt(0).toUpperCase()}
                    </AvatarFallback>
                  )}
                </Avatar>
                <div className="min-w-0 flex-1">
                  <div className="truncate text-sm font-medium text-white">
                    {entry.channel.name}
                  </div>
                  {entry.channel.handle && (
                    <div className="text-xs text-white/50">
                      @{entry.channel.handle}
                    </div>
                  )}
                </div>
                <div className="shrink-0 text-xs text-white/50">
                  {(entry.subscriberCount ?? 0).toLocaleString()} subscribers
                </div>
              </Link>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

export default function SubscriptionsPage() {
  return (
    <Suspense
      fallback={
        <div className="flex items-center justify-center py-24 text-white/50">
          Loading subscriptions…
        </div>
      }
    >
      <SubscriptionsContent />
    </Suspense>
  );
}
