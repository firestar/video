"use client";
import { useState } from "react";
import { cn } from "@/lib/utils";
import { getToken, apiFetch } from "@/lib/api";

interface Props {
  channelId: string;
  initialSubscribed?: boolean;
  initialCount?: number;
}

export default function SubscribeButton({
  channelId,
  initialSubscribed = false,
  initialCount = 0,
}: Props) {
  const [subscribed, setSubscribed] = useState(initialSubscribed);
  const [count, setCount] = useState(initialCount);
  const [pending, setPending] = useState(false);

  const toggle = async () => {
    const next = !subscribed;
    setSubscribed(next);
    setCount((c) => (next ? c + 1 : Math.max(0, c - 1)));

    if (!getToken()) return;
    setPending(true);
    try {
      const res = (await apiFetch<{ subscriberCount: number }>(
        `/api/channels/${channelId}/subscribe`,
        { method: next ? "POST" : "DELETE" }
      )) as { subscriberCount?: number } | undefined;
      if (res?.subscriberCount !== undefined) {
        setCount(res.subscriberCount);
      }
    } catch {
      setSubscribed(!next);
      setCount((c) => (next ? c - 1 : c + 1));
    } finally {
      setPending(false);
    }
  };

  return (
    <button
      type="button"
      onClick={toggle}
      disabled={pending}
      aria-pressed={subscribed}
      className={cn(
        "h-9 shrink-0 rounded-full px-4 text-sm font-medium transition-colors",
        "disabled:cursor-not-allowed disabled:opacity-60",
        subscribed
          ? "bg-muted text-foreground hover:bg-muted/80"
          : "bg-foreground text-background hover:bg-foreground/80"
      )}
    >
      {subscribed ? "Subscribed" : "Subscribe"}
    </button>
  );
}
