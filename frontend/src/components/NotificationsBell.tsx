"use client";
import Link from "next/link";
import { useCallback, useEffect, useRef, useState } from "react";
import { Bell } from "lucide-react";
import { apiFetch } from "@/lib/api";
import { useAuth } from "@/lib/auth";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";

interface Notification {
  id: string;
  channelId: string;
  videoId: string | null;
  type: string;
  readAt: string | null;
  createdAt: string;
  channelName: string;
  channelAvatarUrl: string | null;
  videoTitle: string | null;
  videoThumbnailUrl: string | null;
}

interface NotificationsResponse {
  notifications: Notification[];
  unreadCount: number;
}

const POLL_MS = 30_000;

function timeAgo(iso: string): string {
  const secs = Math.max(0, Math.floor((Date.now() - new Date(iso).getTime()) / 1000));
  if (secs < 60) return "just now";
  const mins = Math.floor(secs / 60);
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days < 7) return `${days}d ago`;
  return new Date(iso).toLocaleDateString();
}
function typeText(type: string): string {
  switch (type) {
    case "new_comment":
      return "commented on your video";
    case "new_like":
      return "liked your video";
    case "new_upload":
    default:
      return "uploaded a video";
  }
}

export default function NotificationsBell() {
  const { user } = useAuth();
  const [open, setOpen] = useState(false);
  const [data, setData] = useState<NotificationsResponse | null>(null);
  const panelRef = useRef<HTMLDivElement>(null);

  const load = useCallback(async () => {
    if (!user) return;
    try {
      const d = await apiFetch<NotificationsResponse>("/api/notifications");
      setData(d);
    } catch {
      // transient error — keep previous data
    }
  }, [user]);

  // Poll while logged in; also refresh when the panel opens.
  useEffect(() => {
    if (!user) {
      setData(null);
      return;
    }
    load();
    const t = setInterval(load, POLL_MS);
    return () => clearInterval(t);
  }, [user, load]);

  // Close on outside click or Escape.
  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (panelRef.current && !panelRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  if (!user) return null;

  const unread = data?.unreadCount ?? 0;

  const markAllRead = async () => {
    try {
      await apiFetch("/api/notifications/read-all", { method: "POST" });
      setData((d) =>
        d
          ? {
              ...d,
              unreadCount: 0,
              notifications: d.notifications.map((n) =>
                n.readAt ? n : { ...n, readAt: new Date().toISOString() }
              ),
            }
          : d
      );
    } catch {
      // ignore
    }
  };

  const openNotification = async (n: Notification) => {
    if (!n.readAt) {
      try {
        const d = await apiFetch<{ unreadCount: number }>(
          `/api/notifications/${n.id}/read`,
          { method: "POST" }
        );
        setData((prev) =>
          prev
            ? {
                ...prev,
                unreadCount: d.unreadCount,
                notifications: prev.notifications.map((x) =>
                  x.id === n.id ? { ...x, readAt: new Date().toISOString() } : x
                ),
              }
            : prev
        );
      } catch {
        // ignore
      }
    }
    setOpen(false);
    if (n.videoId) {
      window.location.href = `/watch/${n.videoId}`;
    }
  };

  return (
    <div className="relative" ref={panelRef}>
      <button
        onClick={() => setOpen((o) => !o)}
        className="relative flex size-9 items-center justify-center rounded-md text-foreground/70 hover:bg-muted hover:text-foreground"
        aria-label={`Notifications${unread > 0 ? ` (${unread} unread)` : ""}`}
      >
        <Bell className="size-5" />
        {unread > 0 && (
          <span className="absolute -right-0.5 -top-0.5 flex h-4 min-w-4 items-center justify-center rounded-full bg-red-600 px-1 text-[10px] font-semibold text-white">
            {unread > 99 ? "99+" : unread}
          </span>
        )}
      </button>

      {open && (
        <div className="absolute right-0 top-11 z-50 w-80 overflow-hidden rounded-lg border border-border bg-background shadow-lg sm:w-96">
          <div className="flex items-center justify-between border-b border-border px-4 py-2.5">
            <span className="text-sm font-semibold">Notifications</span>
            {unread > 0 && (
              <button
                onClick={markAllRead}
                className="text-xs text-muted-foreground hover:text-foreground"
              >
                Mark all as read
              </button>
            )}
          </div>
          <div className="max-h-96 overflow-y-auto">
            {!data || data.notifications.length === 0 ? (
              <div className="px-4 py-10 text-center text-sm text-muted-foreground">
                No notifications yet
              </div>
            ) : (
              data.notifications.map((n) => (
                <button
                  key={n.id}
                  onClick={() => openNotification(n)}
                  className={`flex w-full items-start gap-3 px-4 py-3 text-left hover:bg-muted/50 ${
                    n.readAt ? "" : "bg-muted/30"
                  }`}
                >
                  <Avatar size="default" className="mt-0.5 shrink-0">
                    {n.channelAvatarUrl ? (
                      <AvatarImage src={n.channelAvatarUrl} alt={n.channelName} />
                    ) : (
                      <AvatarFallback>
                        {n.channelName.charAt(0).toUpperCase()}
                      </AvatarFallback>
                    )}
                  </Avatar>
                  <div className="min-w-0 flex-1">
                    <div className="text-sm">
                      <span className="font-medium">{n.channelName}</span>{" "}
                      <span className="text-muted-foreground">{typeText(n.type)}</span>
                    </div>
                    {n.videoTitle && (
                      <div className="mt-0.5 truncate text-sm text-foreground/80">
                        {n.videoTitle}
                      </div>
                    )}
                    <div className="mt-0.5 text-xs text-muted-foreground">
                      {timeAgo(n.createdAt)}
                    </div>
                  </div>
                  {!n.readAt && (
                    <span className="mt-1.5 size-2 shrink-0 rounded-full bg-blue-500" />
                  )}
                </button>
              ))
            )}
          </div>
          {data && data.notifications.length > 0 && (
            <div className="border-t border-border px-4 py-2 text-center">
              <Link
                href="/subscriptions"
                onClick={() => setOpen(false)}
                className="text-xs text-muted-foreground hover:text-foreground"
              >
                View subscribed channels
              </Link>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
