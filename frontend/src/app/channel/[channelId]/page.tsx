"use client";
import { Suspense, useEffect, useState } from "react";
import { useParams } from "next/navigation";
import { apiFetch } from "@/lib/api";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import SubscribeButton from "@/components/SubscribeButton";
import VideoGrid from "@/components/VideoGrid";
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
    return (
      <div className="flex items-center justify-center py-24 text-white/50">
        Loading channel…
      </div>
    );
  }

  if (error || !data) {
    return (
      <div className="rounded-lg border border-red-500/30 bg-red-500/10 p-8 text-sm text-red-300">
        {error ?? "Channel not found"}
      </div>
    );
  }

  const { channel, videos } = data;

  return (
    <div>
      {/* Banner */}
      <div className="h-[200px] w-full overflow-hidden bg-neutral-800">
        {channel.banner_url ? (
          <img
            src={channel.banner_url}
            alt="Channel banner"
            className="h-full w-full object-cover"
          />
        ) : (
          <div className="h-full w-full bg-gradient-to-r from-neutral-800 via-neutral-700 to-neutral-800" />
        )}
      </div>

      {/* Channel header */}
      <div className="mx-auto max-w-6xl px-4">
        <div className="-mt-14 flex flex-col gap-6 sm:flex-row sm:items-end">
          <div className="shrink-0 rounded-full ring-4 ring-[#0f0f0f]">
            <Avatar size="lg" className="size-32">
              {channel.avatar_url ? (
                <AvatarImage src={channel.avatar_url} alt={channel.name} />
              ) : (
                <AvatarFallback className="text-4xl">
                  {channel.name.charAt(0).toUpperCase()}
                </AvatarFallback>
              )}
            </Avatar>
          </div>
          <div className="flex-1 pb-1">
            <h1 className="text-2xl font-bold text-white sm:text-3xl">
              {channel.name}
            </h1>
            {channel.handle && (
              <div className="text-sm text-white/60">@{channel.handle}</div>
            )}
            <div className="mt-1 flex flex-wrap items-center gap-3 text-sm text-white/60">
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
          <p className="mt-4 max-w-3xl text-sm text-white/70">
            {channel.description}
          </p>
        )}

        {/* Videos */}
        <div className="mt-10">
          <h2 className="mb-6 text-lg font-semibold text-white">Videos</h2>
          <VideoGrid videos={videos} />
        </div>
      </div>
    </div>
  );
}

export default function ChannelPage() {
  return (
    <Suspense
      fallback={
        <div className="flex items-center justify-center py-24 text-white/50">
          Loading channel…
        </div>
      }
    >
      <ChannelContent />
    </Suspense>
  );
}
