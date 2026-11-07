"use client";
import { Suspense, useEffect, useState } from "react";
import { useParams } from "next/navigation";
import { apiFetch } from "@/lib/api";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import SubscribeButton from "@/components/SubscribeButton";
import VideoGrid from "@/components/VideoGrid";
import { ChannelPageSkeleton } from "@/components/Skeletons";
import type { Channel, Video } from "@/lib/types";

interface ChannelData {
  channel: Channel;
  subscriberCount: number;
  isSubscribed?: boolean;
  videos: Video[];
}

function ChannelContent() {
  const params = useParams<{ channelId: string }>();
  const channelId = params?.channelId ?? "";

  const [data, setData] = useState<ChannelData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [count, setCount] = useState(0);
  const [subscribed, setSubscribed] = useState(false);

  useEffect(() => {
    if (!channelId) return;
    setLoading(true);
    setError(null);
    apiFetch<ChannelData>(`/api/channels/${channelId}`)
      .then((d) => {
        setData(d);
        setCount(d.subscriberCount ?? 0);
        setSubscribed(d.isSubscribed ?? false);
      })
      .catch((e: Error) => setError(e.message))
      .finally(() => setLoading(false));
  }, [channelId]);

  if (loading) {
    return <ChannelPageSkeleton />;
  }

  if (error || !data) {
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
          <h1 className="text-xl font-semibold">Channel not found</h1>
          <p className="mt-1 text-sm text-muted-foreground">{error}</p>
        </div>
      </div>
    );
  }

  const { channel, videos } = data;

  return (
    <div>
      <div className="h-[160px] w-full overflow-hidden bg-muted sm:h-[200px]">
        {channel.banner_url ? (
          // eslint-disable-next-line @next/next/no-img-element
          <img
            src={channel.banner_url}
            alt="Channel banner"
            className="h-full w-full object-cover"
          />
        ) : (
          <div className="h-full w-full bg-gradient-to-r from-muted via-muted/50 to-muted" />
        )}
      </div>

      <div className="mx-auto max-w-6xl px-4">
        <div className="flex flex-col gap-4 pt-4 sm:-mt-14 sm:flex-row sm:items-end sm:gap-6">
          <div className="shrink-0 rounded-full ring-4 ring-background sm:mt-0 mt-0">
            <Avatar size="lg" className="size-24 sm:size-32">
              {channel.avatar_url ? (
                <AvatarImage src={channel.avatar_url} alt={channel.name} />
              ) : (
                <AvatarFallback className="text-3xl sm:text-4xl">
                  {channel.name.charAt(0).toUpperCase()}
                </AvatarFallback>
              )}
            </Avatar>
          </div>
          <div className="flex-1 pb-1">
            <h1 className="text-xl font-bold sm:text-3xl">{channel.name}</h1>
            {channel.handle && (
              <div className="text-sm text-muted-foreground">@{channel.handle}</div>
            )}
            <div className="mt-1 flex flex-wrap items-center gap-3 text-sm text-muted-foreground">
              <span>{count.toLocaleString()} subscribers</span>
            </div>
          </div>
          <div className="pb-1">
            <SubscribeButton
              key={channel.id}
              channelId={channel.id}
              initialSubscribed={subscribed}
              initialCount={count}
            />
          </div>
        </div>

        {channel.description && (
          <p className="mt-4 max-w-3xl text-sm text-foreground/70">
            {channel.description}
          </p>
        )}

        <div className="mt-10">
          <h2 className="mb-6 text-lg font-semibold">Videos</h2>
          <VideoGrid videos={videos} />
        </div>
      </div>
    </div>
  );
}

export default function ChannelPage() {
  return (
    <Suspense fallback={<ChannelPageSkeleton />}>
      <ChannelContent />
    </Suspense>
  );
}
