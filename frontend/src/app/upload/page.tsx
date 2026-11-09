"use client";
import { useEffect, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { apiFetch } from "@/lib/api";
import { useAuth } from "@/lib/auth";
import type { VideoVisibility } from "@/lib/types";

const MAX_SIZE = 2 * 1024 * 1024 * 1024; // 2GB

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let v = bytes;
  let u = -1;
  do {
    v /= 1024;
    u++;
  } while (v >= 1024 && u < units.length - 1);
  return `${v.toFixed(1)} ${units[u]}`;
}

type Phase = "idle" | "presign" | "uploading" | "creating";

export default function UploadPage() {
  const { user, isLoading } = useAuth();
  const router = useRouter();

  const [file, setFile] = useState<File | null>(null);
  const [dragOver, setDragOver] = useState(false);
  const [fileError, setFileError] = useState<string | null>(null);

  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [tags, setTags] = useState("");
  const [visibility, setVisibility] = useState<VideoVisibility>("public");
  const [schedule, setSchedule] = useState<"now" | "scheduled">("now");
  const [publishAt, setPublishAt] = useState("");
  const [isShort, setIsShort] = useState(false);

  const [phase, setPhase] = useState<Phase>("idle");
  const [progress, setProgress] = useState(0);
  const [error, setError] = useState<string | null>(null);

  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!isLoading && !user) {
      router.push("/login");
    }
  }, [isLoading, user, router]);

  const pickFile = (f: File) => {
    setFileError(null);
    if (!f.type.startsWith("video/")) {
      setFile(null);
      setFileError("Not a video file. Please choose a video.");
      return;
    }
    if (f.size > MAX_SIZE) {
      setFile(null);
      setFileError(`File too large: ${formatBytes(f.size)} (max 2 GB)`);
      return;
    }
    if (!title) setTitle(f.name.replace(/\.[^.]+$/, ""));
    setFile(f);
    setProgress(0);
    setError(null);
  };

  const onDrop = (e: React.DragEvent) => {
    e.preventDefault();
    setDragOver(false);
    const f = e.dataTransfer.files?.[0];
    if (f) pickFile(f);
  };

  const onInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const f = e.target.files?.[0];
    if (f) pickFile(f);
    e.target.value = "";
  };

  const upload = async () => {
    if (!file || !title.trim()) return;
    if (schedule === "scheduled" && (!publishAt || new Date(publishAt) <= new Date())) {
      setError("Pick a future date and time to schedule the publish.");
      return;
    }
    setError(null);
    setProgress(0);
    try {
      setPhase("presign");
      const presign = await apiFetch<{ upload_url: string; object_key: string }>(
        "/api/uploads/presign",
        {
          method: "POST",
          body: JSON.stringify({
            file_name: file.name,
            content_type: file.type,
            size: file.size,
          }),
        }
      );

      setPhase("uploading");
      await new Promise<void>((resolve, reject) => {
        const xhr = new XMLHttpRequest();
        xhr.open("PUT", presign.upload_url);
        xhr.setRequestHeader("Content-Type", file.type);
        xhr.upload.onprogress = (e) => {
          if (e.lengthComputable) {
            setProgress(Math.round((e.loaded / e.total) * 100));
          }
        };
        xhr.onload = () => {
          if (xhr.status >= 200 && xhr.status < 300) resolve();
          else reject(new Error(`Upload failed (HTTP ${xhr.status})`));
        };
        xhr.onerror = () => reject(new Error("Upload failed (network error)"));
        xhr.send(file);
      });

      setPhase("creating");
      const created = await apiFetch<{ video: { id: string } }>("/api/videos", {
        method: "POST",
        body: JSON.stringify({
          title: title.trim(),
          description: description.trim() || null,
          tags: tags
            .split(",")
            .map((t) => t.trim())
            .filter(Boolean),
          object_key: presign.object_key,
          visibility,
          isShort,
          ...(schedule === "scheduled"
            ? { publishAt: new Date(publishAt).toISOString() }
            : {}),
        }),
      });

      router.push(`/watch/${created.video.id}`);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : "Upload failed");
      setPhase("idle");
    }
  };

  const busy = phase !== "idle";

  if (isLoading || !user) {
    return (
      <div className="flex items-center justify-center py-24 text-muted-foreground">
        Checking session…
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-2xl">
      <h1 className="mb-6 text-2xl font-semibold">Upload</h1>

      {!file ? (
        <div
          onDragOver={(e) => {
            e.preventDefault();
            setDragOver(true);
          }}
          onDragLeave={() => setDragOver(false)}
          onDrop={onDrop}
          onClick={() => inputRef.current?.click()}
          className={`flex cursor-pointer flex-col items-center justify-center gap-3 rounded-lg border-2 border-dashed p-16 text-center transition-colors ${
            dragOver
              ? "border-blue-400 bg-blue-500/10"
              : "border-border bg-muted hover:border-foreground/40"
          }`}
        >
          <div className="text-5xl">📹</div>
          <p className="text-lg font-medium">Drag & drop your video here</p>
          <p className="text-sm text-muted-foreground">or click to browse · video files up to 2 GB</p>
          <input
            ref={inputRef}
            type="file"
            accept="video/*"
            className="hidden"
            onChange={onInputChange}
          />
        </div>
      ) : (
        <div className="flex items-center justify-between rounded-lg border border-border bg-muted p-4">
          <div className="min-w-0">
            <p className="truncate font-medium">{file.name}</p>
            <p className="text-sm text-muted-foreground">{formatBytes(file.size)}</p>
          </div>
          <Button
            variant="ghost"
            size="sm"
            disabled={busy}
            onClick={() => {
              setFile(null);
              setProgress(0);
            }}
          >
            Remove
          </Button>
        </div>
      )}
      {fileError && <p className="mt-2 text-sm text-destructive">{fileError}</p>}

      <Card className="mt-6">
        <CardHeader>
          <CardTitle>Video details</CardTitle>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          <Input
            placeholder="Title (required)"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            disabled={busy}
            required
          />
          <textarea
            placeholder="Description"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            disabled={busy}
            rows={4}
            className="w-full rounded-md border border-border bg-transparent px-3 py-2 text-sm placeholder:text-muted-foreground focus:outline-none"
          />
          <Input
            placeholder="Tags (comma-separated)"
            value={tags}
            onChange={(e) => setTags(e.target.value)}
            disabled={busy}
          />
          <div>
            <p className="mb-2 text-sm font-medium">Visibility</p>
            <div className="flex flex-col gap-2">
              {(
                [
                  ["public", "Public", "Anyone can find this video on synload and watch it"],
                  ["unlisted", "Unlisted", "Anyone with the link can watch this video; it won't appear in listings or search"],
                  ["private", "Private", "Only you can watch this video"],
                ] as [VideoVisibility, string, string][]
              ).map(([value, label, hint]) => (
                <label
                  key={value}
                  className={`flex cursor-pointer items-start gap-3 rounded-md border p-3 text-sm transition-colors ${
                    visibility === value
                      ? "border-blue-500 bg-blue-500/10"
                      : "border-border hover:border-foreground/40"
                  }`}
                >
                  <input
                    type="radio"
                    name="visibility"
                    value={value}
                    checked={visibility === value}
                    onChange={() => setVisibility(value)}
                    disabled={busy}
                    className="mt-0.5 accent-blue-500"
                  />
                  <span>
                    <span className="font-medium">{label}</span>
                    <span className="block text-muted-foreground">{hint}</span>
                  </span>
                </label>
              ))}
            </div>
          </div>
          <div>
            <p className="mb-2 text-sm font-medium">Publish</p>
            <div className="flex flex-col gap-2">
              {(
                [
                  ["now", "Publish now", "The video goes live as soon as it finishes processing"],
                  ["scheduled", "Schedule publish", "Pick a date and time; the video goes live automatically then"],
                ] as ["now" | "scheduled", string, string][]
              ).map(([value, label, hint]) => (
                <label
                  key={value}
                  className={`flex cursor-pointer items-start gap-3 rounded-md border p-3 text-sm transition-colors ${
                    schedule === value
                      ? "border-blue-500 bg-blue-500/10"
                      : "border-border hover:border-foreground/40"
                  }`}
                >
                  <input
                    type="radio"
                    name="schedule"
                    value={value}
                    checked={schedule === value}
                    onChange={() => setSchedule(value)}
                    disabled={busy}
                    className="mt-0.5 accent-blue-500"
                  />
                  <span>
                    <span className="font-medium">{label}</span>
                    <span className="block text-muted-foreground">{hint}</span>
                  </span>
                </label>
              ))}
              {schedule === "scheduled" && (
                <Input
                  type="datetime-local"
                  value={publishAt}
                  onChange={(e) => setPublishAt(e.target.value)}
                  disabled={busy}
                  className="mt-1 max-w-xs"
                />
              )}
            </div>
          </div>
          <div>
            <p className="mb-2 text-sm font-medium">Format</p>
            <label className="flex cursor-pointer items-start gap-3 rounded-md border p-3 text-sm transition-colors">
              <input
                type="checkbox"
                checked={isShort}
                onChange={(e) => setIsShort(e.target.checked)}
                disabled={busy}
                className="mt-0.5 accent-blue-500"
              />
              <span>
                <span className="font-medium">Short</span>
                <span className="block text-muted-foreground">Vertical, under 60 seconds. Appears in the Shorts feed.</span>
              </span>
            </label>
          </div>
          {phase === "uploading" && (
            <div>
              <div className="h-2 w-full overflow-hidden rounded-full bg-muted">
                <div
                  className="h-full bg-blue-500 transition-all"
                  style={{ width: `${progress}%` }}
                />
              </div>
              <p className="mt-1 text-sm text-muted-foreground">{progress}%</p>
            </div>
          )}
          {phase === "presign" && (
            <p className="text-sm text-muted-foreground">Preparing upload…</p>
          )}
          {phase === "creating" && (
            <p className="text-sm text-muted-foreground">Upload complete. Finalizing…</p>
          )}

          {error && <p className="text-sm text-destructive">{error}</p>}

          <Button
            onClick={upload}
            disabled={busy || !file || !title.trim()}
            className="w-full"
          >
            {phase === "uploading"
              ? `Uploading… ${progress}%`
              : phase === "creating"
                ? "Finalizing…"
                : phase === "presign"
                  ? "Preparing…"
                  : "Upload"}
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}
