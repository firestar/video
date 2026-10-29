"use client";
import { useEffect, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { apiFetch } from "@/lib/api";
import { useAuth } from "@/lib/auth";

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
    setError(null);
    setProgress(0);
    try {
      // 1. Presign
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

      // 2. PUT to presigned URL with progress (XHR, not fetch)
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

      // 3. Create video record
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
        }),
      });

      // 4. Redirect
      router.push(`/watch/${created.video.id}`);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : "Upload failed");
      setPhase("idle");
    }
  };

  const busy = phase !== "idle";

  if (isLoading || !user) {
    return (
      <div className="flex items-center justify-center py-24 text-white/50">
        Checking session…
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-2xl">
      <h1 className="mb-6 text-2xl font-semibold">Upload</h1>

      {/* Drop area */}
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
              : "border-white/20 bg-white/5 hover:border-white/40"
          }`}
        >
          <div className="text-5xl">📹</div>
          <p className="text-lg font-medium text-white">
            Drag & drop your video here
          </p>
          <p className="text-sm text-white/50">or click to browse · video files up to 2 GB</p>
          <input
            ref={inputRef}
            type="file"
            accept="video/*"
            className="hidden"
            onChange={onInputChange}
          />
        </div>
      ) : (
        <div className="flex items-center justify-between rounded-lg border border-white/15 bg-white/5 p-4">
          <div className="min-w-0">
            <p className="truncate font-medium text-white">{file.name}</p>
            <p className="text-sm text-white/50">{formatBytes(file.size)}</p>
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
      {fileError && <p className="mt-2 text-sm text-red-400">{fileError}</p>}

      {/* Fields */}
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
            className="w-full rounded-md border border-white/20 bg-transparent px-3 py-2 text-sm text-white placeholder:text-white/40 focus:outline-none"
          />
          <Input
            placeholder="Tags (comma-separated)"
            value={tags}
            onChange={(e) => setTags(e.target.value)}
            disabled={busy}
          />

          {/* Progress */}
          {phase === "uploading" && (
            <div>
              <div className="h-2 w-full overflow-hidden rounded-full bg-white/10">
                <div
                  className="h-full bg-blue-500 transition-all"
                  style={{ width: `${progress}%` }}
                />
              </div>
              <p className="mt-1 text-sm text-white/50">{progress}%</p>
            </div>
          )}
          {phase === "presign" && (
            <p className="text-sm text-white/50">Preparing upload…</p>
          )}
          {phase === "creating" && (
            <p className="text-sm text-white/50">Upload complete. Finalizing…</p>
          )}

          {error && <p className="text-sm text-red-400">{error}</p>}

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
