"use client";
import { Suspense, useCallback, useEffect, useState } from "react";
import { useParams, useRouter } from "next/navigation";
import { ListVideo, Pencil, Trash2, X } from "lucide-react";
import { apiFetch, getToken } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import VideoCard from "@/components/VideoCard";
import EmptyState from "@/components/EmptyState";
import ErrorState from "@/components/ErrorState";
import { ListSkeleton } from "@/components/Skeletons";
import { formatCount, formatDate } from "@/lib/format";
import type { Playlist, Video } from "@/lib/types";

interface Detail {
  playlist: Playlist;
  videos: Video[];
}

function PlaylistContent() {
  const params = useParams<{ playlistId: string }>();
  const router = useRouter();
  const playlistId = params?.playlistId ?? "";
  const [data, setData] = useState<Detail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [editing, setEditing] = useState(false);
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [isPublic, setIsPublic] = useState(false);
  const [busy, setBusy] = useState(false);
  const [currentUser, setCurrentUser] = useState<string>("");

  const load = useCallback(async () => {
    if (!playlistId) return;
    setLoading(true);
    setError(null);
    try {
      const d = await apiFetch<Detail>(`/api/playlists/${playlistId}`);
      setData(d);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load playlist");
    } finally {
      setLoading(false);
    }
  }, [playlistId]);

  useEffect(() => {
    load();
  }, [load]);

  useEffect(() => {
    if (!getToken()) return;
    apiFetch<{ id: string }>("/api/me")
      .then((u) => setCurrentUser(u.id))
      .catch(() => {});
  }, []);

  const isOwner = !!data && data.playlist.userId === currentUser;

  const startEdit = () => {
    if (!data) return;
    setTitle(data.playlist.title);
    setDescription(data.playlist.description);
    setIsPublic(data.playlist.isPublic);
    setEditing(true);
  };

  const saveEdit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!data) return;
    const t = title.trim();
    if (!t) return;
    setBusy(true);
    try {
      await apiFetch(`/api/playlists/${data.playlist.id}`, {
        method: "PATCH",
        body: JSON.stringify({
          title: t,
          description: description.trim(),
          isPublic,
        }),
      });
      setEditing(false);
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to update playlist");
    } finally {
      setBusy(false);
    }
  };

  const removeVideo = async (videoId: string) => {
    if (!data) return;
    try {
      await apiFetch(
        `/api/playlists/${data.playlist.id}/items/${videoId}`,
        { method: "DELETE" }
      );
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to remove video");
    }
  };

  const deletePlaylist = async () => {
    if (!data) return;
    if (!confirm("Delete this playlist?")) return;
    setBusy(true);
    try {
      await apiFetch(`/api/playlists/${data.playlist.id}`, {
        method: "DELETE",
      });
      router.push("/playlists");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to delete playlist");
      setBusy(false);
    }
  };

  if (loading) {
    return (
      <div className="mx-auto max-w-6xl px-4 py-6">
        <h1 className="mb-6 text-xl font-semibold">Playlist</h1>
        <ListSkeleton count={4} />
      </div>
    );
  }

  if (error || !data) {
    return (
      <div className="mx-auto max-w-6xl px-4 py-6">
        {error ? (
          <ErrorState title="Playlist not found" message={error} onRetry={load} />
        ) : (
          <EmptyState
            icon={<ListVideo className="size-8 text-muted-foreground" />}
            title="Playlist not found"
          />
        )}
      </div>
    );
  }

  const { playlist, videos } = data;

  return (
    <div className="mx-auto max-w-6xl px-4 py-6">
      <div className="mb-6 flex flex-col gap-4 rounded-lg border border-border bg-muted/30 p-5">
        {editing ? (
          <form onSubmit={saveEdit} className="flex flex-col gap-3">
            <Input
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="Playlist title"
              autoFocus
            />
            <Input
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="Description (optional)"
            />
            <label className="flex items-center gap-2 text-sm text-foreground/80">
              <input
                type="checkbox"
                checked={isPublic}
                onChange={(e) => setIsPublic(e.target.checked)}
                className="size-4"
              />
              Public
            </label>
            <div className="flex justify-end gap-2">
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => setEditing(false)}
              >
                Cancel
              </Button>
              <Button type="submit" size="sm" disabled={busy || !title.trim()}>
                {busy ? "Saving…" : "Save"}
              </Button>
            </div>
          </form>
        ) : (
          <>
            <div className="flex items-start justify-between gap-4">
              <div className="min-w-0">
                <h1 className="text-2xl font-bold">{playlist.title}</h1>
                {playlist.description && (
                  <p className="mt-1 text-sm text-muted-foreground">
                    {playlist.description}
                  </p>
                )}
                <div className="mt-2 text-sm text-muted-foreground/80">
                  {playlist.videoCount} video{playlist.videoCount === 1 ? "" : "s"} ·{" "}
                  {playlist.isPublic ? "Public" : "Private"} ·{" "}
                  {formatDate(playlist.updatedAt)}
                </div>
              </div>
              {isOwner && (
                <div className="flex shrink-0 gap-2">
                  <Button variant="outline" size="sm" onClick={startEdit}>
                    <Pencil className="size-4" />
                    Edit
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={deletePlaylist}
                    disabled={busy}
                  >
                    <Trash2 className="size-4" />
                    Delete
                  </Button>
                </div>
              )}
            </div>
          </>
        )}
      </div>

      {videos.length === 0 ? (
        <EmptyState
          icon={<ListVideo className="size-8 text-muted-foreground" />}
          title="No videos in this playlist"
          subtitle={
            isOwner
              ? "Add videos from the watch page using the “Add to playlist” button."
              : "This playlist is empty."
          }
        />
      ) : (
        <div className="grid grid-cols-1 gap-x-4 gap-y-8 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4">
          {videos.map((v) => (
            <div key={v.id} className="group relative">
              <VideoCard video={v} />
              {isOwner && (
                <button
                  onClick={() => removeVideo(v.id)}
                  className="absolute right-2 top-2 z-10 flex size-8 items-center justify-center rounded-full bg-black/70 text-white opacity-0 transition-opacity hover:bg-black group-hover:opacity-100"
                  aria-label="Remove from playlist"
                >
                  <X className="size-4" />
                </button>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export default function PlaylistPage() {
  return (
    <Suspense
      fallback={
        <div className="mx-auto max-w-6xl px-4 py-6">
          <h1 className="mb-6 text-xl font-semibold">Playlist</h1>
          <ListSkeleton count={4} />
        </div>
      }
    >
      <PlaylistContent />
    </Suspense>
  );
}
