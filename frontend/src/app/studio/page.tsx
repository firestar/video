"use client";
import { useEffect, useState } from "react";
import { apiFetch } from "@/lib/api";
import { useAuth } from "@/lib/auth";
import { Button } from "@/components/ui/button";
import type { Video, VideoVisibility } from "@/lib/types";

interface MeResponse {
  id: string;
  email: string;
  name: string;
  channel: string;
  channelId?: string;
}

function formatDate(iso?: string) {
  if (!iso) return "—";
  const d = new Date(iso);
  return d.toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" });
}

function formatDuration(secs?: number) {
  if (secs == null) return "—";
  const m = Math.floor(secs / 60);
  const s = secs % 60;
  return `${m}:${s.toString().padStart(2, "0")}`;
}

function StudioContent() {
  const { user, logout } = useAuth();
  const [videos, setVideos] = useState<Video[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [channelId, setChannelId] = useState<string | null>(null);

  // Load the user's channel id, then their videos.
  useEffect(() => {
    if (!user) {
      setLoading(false);
      return;
    }
    setLoading(true);
    setError(null);
    apiFetch<MeResponse>("/api/me")
      .then((me) => {
        setChannelId(me.channelId ?? null);
        if (!me.channelId) return;
        return apiFetch<Video[]>(`/api/channels/${me.channelId}/videos?channelId=${me.channelId}`);
      })
      .then((rows) => {
        if (rows) setVideos(rows);
      })
      .catch((e: Error) => setError(e.message))
      .finally(() => setLoading(false));
  }, [user]);

  const [editing, setEditing] = useState<string | null>(null);
  const [draft, setDraft] = useState({ title: "", description: "", tags: "", visibility: "public" as VideoVisibility });
  const [saving, setSaving] = useState(false);
  const [savedMsg, setSavedMsg] = useState<string | null>(null);
  const [deleting, setDeleting] = useState<string | null>(null);

  const startEdit = (v: Video) => {
    setEditing(v.id);
    setDraft({
      title: v.title,
      description: v.description ?? "",
      tags: (v.tags ?? []).join(", "),
      visibility: v.visibility ?? "public",
    });
    setSavedMsg(null);
  };

  const cancelEdit = () => {
    setEditing(null);
    setSavedMsg(null);
  };

  const saveEdit = async (v: Video) => {
    setSaving(true);
    setError(null);
    setSavedMsg(null);
    try {
      const body = {
        title: draft.title,
        description: draft.description,
        tags: draft.tags
          .split(",")
          .map((t) => t.trim())
          .filter(Boolean),
        visibility: draft.visibility,
      };
      const updated: Video = await apiFetch(`/api/videos/${v.id}`, {
        method: "PATCH",
        body: JSON.stringify(body),
      });
      setVideos((prev) =>
        prev.map((x) => (x.id === v.id ? { ...x, ...updated } : x))
      );
      setEditing(null);
      setSavedMsg("Video updated");
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setSaving(false);
    }
  };

  const deleteVideo = async (v: Video) => {
    setDeleting(v.id);
    setError(null);
    try {
      await apiFetch(`/api/videos/${v.id}`, { method: "DELETE" });
      setVideos((prev) => prev.filter((x) => x.id !== v.id));
      if (editing === v.id) {
        setEditing(null);
      }
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setDeleting(null);
    }
  };

  if (!user) {
    return (
      <div className="flex flex-col items-center gap-4 py-24 text-center">
        <h1 className="text-xl font-semibold">Studio</h1>
        <p className="text-sm text-muted-foreground">
          Log in to manage your videos.
        </p>
        <Button onClick={() => {
          window.location.href = "/login";
        }}>
          Go to Login
        </Button>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center py-24">
        <div className="flex items-center gap-3 text-sm text-muted-foreground">
          <div className="size-5 animate-spin rounded-full border-2 border-border border-t-foreground" />
          Loading your videos…
        </div>
      </div>
    );
  }

  if (error && videos.length === 0) {
    return (
      <div className="flex flex-col items-center gap-4 py-24 text-center">
        <h1 className="text-xl font-semibold">Studio</h1>
        <p className="text-sm text-muted-foreground">{error}</p>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-5xl">
      <div className="mb-6">
        <h1 className="text-2xl font-bold">Studio</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Manage your channel's videos — edit titles, descriptions, and tags, or delete videos.
        </p>
      </div>

      {savedMsg && (
        <div className="mb-4 rounded-md border border-border bg-muted/40 px-4 py-3 text-sm text-foreground/80">
          {savedMsg}
        </div>
      )}
      {error && videos.length > 0 && (
        <div className="mb-4 rounded-md border border-border bg-muted/40 px-4 py-3 text-sm text-foreground/80">
          {error}
        </div>
      )}

      {videos.length === 0 ? (
        <div className="flex flex-col items-center gap-4 py-24 text-center">
          <div className="flex size-16 items-center justify-center rounded-full bg-muted">
            <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="text-muted-foreground">
              <rect x="3" y="5" width="18" height="14" rx="2" />
              <path d="M3 9h18" />
              <circle cx="12" cy="14" r="2" />
            </svg>
          </div>
          <div>
            <h2 className="text-lg font-semibold">No videos yet</h2>
            <p className="mt-1 text-sm text-muted-foreground">
              Upload a video to get started.
            </p>
          </div>
        </div>
      ) : (
        <div className="flex flex-col gap-3">
          {videos.map((v) => {
            const isEditing = editing === v.id;
            return (
              <div key={v.id} className="flex items-center gap-4 rounded-lg border border-border bg-background p-3">
                <div className="size-24 shrink-0 overflow-hidden rounded-md bg-muted sm:size-28">
                  {v.thumbnailUrl ? (
                    // eslint-disable-next-line @next/next/no-img-element
                    <img src={v.thumbnailUrl} alt={v.title} className="h-full w-full object-cover" />
                  ) : (
                    <div className="h-full w-full flex items-center justify-center text-muted-foreground">
                      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                        <rect x="3" y="5" width="18" height="14" rx="2" />
                        <path d="M3 9h18" />
                        <circle cx="12" cy="14" r="2" />
                      </svg>
                    </div>
                  )}
                </div>

                <div className="flex-1 min-w-0">
                  {isEditing ? (
                    <div className="flex flex-col gap-3">
                      <div>
                        <label className="text-xs font-medium text-muted-foreground">Title</label>
                        <input
                          type="text"
                          value={draft.title}
                          onChange={(e) => setDraft({ ...draft, title: e.target.value })}
                          className="mt-1 h-9 w-full rounded-md border border-border bg-background px-3 text-sm outline-none focus:border-ring focus:ring-3 focus:ring-ring/30"
                        />
                      </div>
                      <div>
                        <label className="text-xs font-medium text-muted-foreground">Description</label>
                        <textarea
                          value={draft.description}
                          onChange={(e) => setDraft({ ...draft, description: e.target.value })}
                          rows={2}
                          className="mt-1 w-full rounded-md border border-border bg-background px-3 py-2 text-sm outline-none focus:border-ring focus:ring-3 focus:ring-ring/30"
                        />
                      </div>
                      <div>
                        <label className="text-xs font-medium text-muted-foreground">Tags (comma-separated)</label>
                        <input
                          type="text"
                          value={draft.tags}
                          onChange={(e) => setDraft({ ...draft, tags: e.target.value })}
                          placeholder="e2e, sample, demo"
                          className="mt-1 h-9 w-full rounded-md border border-border bg-background px-3 text-sm outline-none focus:border-ring focus:ring-3 focus:ring-ring/30"
                        />
                      </div>
                      <div>
                        <label className="text-xs font-medium text-muted-foreground">Visibility</label>
                        <select
                          value={draft.visibility}
                          onChange={(e) => setDraft({ ...draft, visibility: e.target.value as VideoVisibility })}
                          className="mt-1 h-9 w-full rounded-md border border-border bg-background px-3 text-sm outline-none focus:border-ring focus:ring-3 focus:ring-ring/30"
                        >
                          <option value="public">Public — anyone can find and watch</option>
                          <option value="unlisted">Unlisted — anyone with the link</option>
                          <option value="private">Private — only you</option>
                        </select>
                      </div>
                       <div className="flex items-center gap-2">
                         <Button size="sm" onClick={() => saveEdit(v)} disabled={saving}>
                          {saving ? "Saving…" : "Save changes"}
                        </Button>
                        <Button size="sm" variant="outline" onClick={cancelEdit}>
                          Cancel
                        </Button>
                      </div>
                    </div>
                  ) : (
                    <div className="flex items-start justify-between gap-3">
                      <div className="min-w-0">
                        <h3 className="truncate text-sm font-medium">{v.title}</h3>
                          <div className="mt-0.5 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground">
                            <span className={
                              v.visibility === "private"
                                ? "font-medium text-amber-500"
                                : v.visibility === "unlisted"
                                  ? "font-medium text-blue-500"
                                  : ""
                            }>
                              {v.visibility ?? "public"}
                            </span>
                            <span>{v.status}</span>
                          <span>{formatDuration(v.durationSecs)}</span>
                          <span>{v.viewCount.toLocaleString()} views</span>
                          <span>{v.likeCount.toLocaleString()} likes</span>
                           {v.publishedAt && <span>{formatDate(v.publishedAt)}</span>}
                        </div>
                        {v.tags && v.tags.length > 0 && (
                          <div className="mt-1 flex flex-wrap gap-1">
                            {v.tags.map((t) => (
                              <span key={t} className="rounded bg-muted px-1.5 py-0.5 text-xs text-foreground/80">
                                {t}
                              </span>
                            ))}
                          </div>
                        )}
                      </div>
                      <div className="flex shrink-0 items-center gap-2">
                        <Button size="sm" variant="outline" onClick={() => startEdit(v)}>
                          Edit
                        </Button>
                        <Button size="sm" variant="ghost" onClick={() => deleteVideo(v)} disabled={deleting === v.id}>
                          {deleting === v.id ? "Deleting…" : "Delete"}
                        </Button>
                      </div>
                    </div>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

export default function StudioPage() {
  return <StudioContent />;
}
