"use client";
import { Suspense, useCallback, useEffect, useState } from "react";
import { ListVideo, Plus } from "lucide-react";
import { apiFetch, getToken } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import EmptyState from "@/components/EmptyState";
import ErrorState from "@/components/ErrorState";
import { ListSkeleton } from "@/components/Skeletons";
import { formatDate } from "@/lib/format";
import type { Playlist } from "@/lib/types";

function PlaylistsContent() {
  const [playlists, setPlaylists] = useState<Playlist[] | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [isPublic, setIsPublic] = useState(false);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await apiFetch<{ playlists: Playlist[] }>("/api/playlists");
      setPlaylists(data.playlists);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load playlists");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (!getToken()) {
      setPlaylists([]);
      setLoading(false);
      return;
    }
    load();
  }, [load]);

  const create = async (e: React.FormEvent) => {
    e.preventDefault();
    const t = title.trim();
    if (!t) return;
    setBusy(true);
    try {
      await apiFetch("/api/playlists", {
        method: "POST",
        body: JSON.stringify({
          title: t,
          description: description.trim(),
          isPublic,
        }),
      });
      setTitle("");
      setDescription("");
      setIsPublic(false);
      setShowForm(false);
      await load();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to create playlist");
    } finally {
      setBusy(false);
    }
  };

  if (loading) {
    return (
      <div className="mx-auto max-w-6xl px-4 py-6">
        <h1 className="mb-6 text-xl font-semibold">Playlists</h1>
        <ListSkeleton count={4} />
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-6xl px-4 py-6">
      <div className="mb-6 flex items-center justify-between">
        <h1 className="text-xl font-semibold">Playlists</h1>
        <Button
          size="sm"
           onClick={() => {
             setShowForm((s) => !s);
           }}
        >
          <Plus className="size-4" />
          New playlist
        </Button>
      </div>

      {showForm && (
        <form
          onSubmit={create}
          className="mb-6 flex flex-col gap-3 rounded-lg border border-border bg-muted/30 p-4"
        >
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
            Public (anyone with the link can view)
          </label>
          <div className="flex justify-end gap-2">
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={() => setShowForm(false)}
            >
              Cancel
            </Button>
            <Button type="submit" size="sm" disabled={busy || !title.trim()}>
              {busy ? "Creating…" : "Create"}
            </Button>
          </div>
        </form>
      )}

      {error ? (
        <ErrorState title="Can't connect" message={error} onRetry={load} />
      ) : playlists && playlists.length === 0 ? (
        <EmptyState
          icon={<ListVideo className="size-8 text-muted-foreground" />}
          title="No playlists yet"
          subtitle="Create a playlist to organize your favorite videos."
        />
      ) : (
        <ul className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
          {playlists?.map((p) => (
            <li key={p.id}>
              <a
                href={`/playlists/${p.id}`}
                className="group flex flex-col gap-2 rounded-lg border border-border p-4 transition-colors hover:bg-muted"
              >
                <div className="flex items-center gap-2">
                  <ListVideo className="size-5 text-muted-foreground" />
                  <span className="truncate font-medium">{p.title}</span>
                </div>
                {p.description && (
                  <p className="line-clamp-2 text-sm text-muted-foreground">
                    {p.description}
                  </p>
                )}
                <div className="text-xs text-muted-foreground/70">
                  {p.videoCount} video{p.videoCount === 1 ? "" : "s"} ·{" "}
                  {p.isPublic ? "Public" : "Private"} · {formatDate(p.updatedAt)}
                </div>
              </a>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

export default function PlaylistsPage() {
  return (
    <Suspense
      fallback={
        <div className="mx-auto max-w-6xl px-4 py-6">
          <h1 className="mb-6 text-xl font-semibold">Playlists</h1>
          <ListSkeleton count={4} />
        </div>
      }
    >
      <PlaylistsContent />
    </Suspense>
  );
}
