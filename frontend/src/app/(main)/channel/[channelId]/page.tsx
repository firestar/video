"use client";
import { Suspense, useEffect, useState } from "react";
import { useParams } from "next/navigation";
import { apiFetch } from "@/lib/api";
import { useAuth } from "@/lib/auth";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import SubscribeButton from "@/components/SubscribeButton";
import ChannelEditor from "@/components/ChannelEditor";
import VideoGrid from "@/components/VideoGrid";
import { ChannelPageSkeleton } from "@/components/Skeletons";
import type { Channel, Video } from "@/lib/types";
import { formatCount, formatDate } from "@/lib/format";
interface ChannelData {
  channel: Channel;
  subscriberCount: number;
  isSubscribed?: boolean;
  videoCount?: number;
  totalViews?: number;
  videos: Video[];
}

function ChannelContent() {
  const params = useParams<{ channelId: string }>();
  const channelId = params?.channelId ?? "";
  const { user } = useAuth();

  const [data, setData] = useState<ChannelData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [count, setCount] = useState(0);
  const [subscribed, setSubscribed] = useState(false);
  const [editing, setEditing] = useState(false);
  const [tab, setTab] = useState<"videos" | "about">("videos");
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
        {channel.bannerUrl ? (
          // eslint-disable-next-line @next/next/no-img-element
          <img
            src={channel.bannerUrl}
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
              {channel.avatarUrl ? (
                <AvatarImage src={channel.avatarUrl} alt={channel.name} />
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
          <div className="flex items-center gap-2 pb-1">
            {user && channel.userId === user.id && (
              <Button variant="outline" size="sm" onClick={() => setEditing(true)}>
                Edit channel
              </Button>
            )}
            <SubscribeButton
              key={channel.id}
              channelId={channel.id}
              initialSubscribed={subscribed}
              initialCount={count}
            />
          </div>
        </div>

        {editing && (
          <ChannelEditor
            channel={channel}
            onSaved={(updated) =>
              setData((d) => (d ? { ...d, channel: updated } : d))
            }
            onClose={() => setEditing(false)}
          />
        )}

        <div className="mt-6 flex gap-6 border-b border-border">
          {(["videos", "about"] as const).map((t) => (
            <button
              key={t}
              onClick={() => setTab(t)}
              className={`-mb-px border-b-2 px-1 pb-3 text-sm font-medium capitalize transition-colors ${
                tab === t
                  ? "border-foreground text-foreground"
                  : "border-transparent text-muted-foreground hover:text-foreground"
              }`}
            >
              {t}
            </button>
          ))}
        </div>

        {tab === "videos" ? (
          <div className="mt-8">
            <VideoGrid videos={videos} />
          </div>
        ) : (
          <div className="mt-6 max-w-3xl space-y-4 text-sm">
            {channel.description ? (
              <p className="whitespace-pre-wrap text-foreground/80">{channel.description}</p>
            ) : (
              <p className="text-muted-foreground">No description provided.</p>
            )}
            <div className="space-y-1 pt-2 text-muted-foreground">
              <div><span className="font-medium text-foreground">{count.toLocaleString()}</span> subscribers</div>
              <div><span className="font-medium text-foreground">{(data.videoCount ?? 0).toLocaleString()}</span> {(data.videoCount ?? 0) === 1 ? "video" : "videos"}</div>
              <div><span className="font-medium text-foreground">{formatCount(data.totalViews ?? 0)}</span> total views</div>
              <div>Joined {formatDate(channel.createdAt)}</div>
            </div>
            <div className="pt-2">
              <div className="text-xs uppercase tracking-wide text-muted-foreground">Channel URL</div>
              <a href={`/channel/${channel.id}`} className="break-all text-primary hover:underline">
                video.synload.com/channel/{channel.id}
              </a>
            </div>
          </div>
        )}
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
