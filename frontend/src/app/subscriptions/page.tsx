"use client";
import { Suspense, useEffect, useState } from "react";
import Link from "next/link";
import { Users } from "lucide-react";
import { apiFetch, getToken } from "@/lib/api";
import EmptyState from "@/components/EmptyState";
import ErrorState from "@/components/ErrorState";
import { ListSkeleton } from "@/components/Skeletons";
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

  const load = () => {
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
  };

  useEffect(() => {
    load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  if (loading) {
    return (
      <div className="mx-auto max-w-6xl px-4 py-6">
        <h1 className="mb-6 text-xl font-semibold">Subscriptions</h1>
        <ListSkeleton count={5} />
      </div>
    );
  }

  if (error) {
    return (
      <div className="mx-auto max-w-6xl px-4 py-6">
        <h1 className="mb-6 text-xl font-semibold">Subscriptions</h1>
        <ErrorState title="Can't connect" message={error} onRetry={load} />
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-6xl px-4 py-6">
      <h1 className="mb-6 text-xl font-semibold">Subscriptions</h1>

      {items && items.length === 0 ? (
        <EmptyState
          icon={<Users className="size-8 text-muted-foreground" />}
          title="No subscriptions yet"
          subtitle="Subscribe to channels to see their videos here."
        />
      ) : (
        <ul className="flex flex-col gap-2">
          {items?.map((entry) => (
            <li key={entry.channel.id}>
              <Link
                href={`/channel/${entry.channel.id}`}
                className="flex items-center gap-4 rounded-lg p-2 hover:bg-muted"
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
                  <div className="truncate text-sm font-medium">
                    {entry.channel.name}
                  </div>
                  {entry.channel.handle && (
                    <div className="text-xs text-muted-foreground">
                      @{entry.channel.handle}
                    </div>
                  )}
                </div>
                <div className="shrink-0 text-xs text-muted-foreground">
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
        <div className="mx-auto max-w-6xl px-4 py-6">
          <h1 className="mb-6 text-xl font-semibold">Subscriptions</h1>
          <ListSkeleton count={5} />
        </div>
      }
    >
      <SubscriptionsContent />
    </Suspense>
  );
}
