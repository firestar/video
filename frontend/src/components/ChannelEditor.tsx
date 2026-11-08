"use client";
import { useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { apiFetch, mediaUrl } from "@/lib/api";
import type { Channel } from "@/lib/types";

const MAX_AVATAR = 5 * 1024 * 1024; // 5 MB
const MAX_BANNER = 10 * 1024 * 1024; // 10 MB

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB"];
  let v = bytes;
  let u = -1;
  do {
    v /= 1024;
    u++;
  } while (v >= 1024 && u < units.length - 1);
  return `${v.toFixed(1)} ${units[u]}`;
}

interface Props {
  channel: Channel;
  onSaved: (channel: Channel) => void;
  onClose: () => void;
}

/**
 * Owner-only channel editor: name, description, avatar and banner.
 * Images go presign (purpose=avatar|banner) -> direct PUT -> PATCH with the
 * public /media/ URL.
 */
export default function ChannelEditor({ channel, onSaved, onClose }: Props) {
  const [name, setName] = useState(channel.name);
  const [description, setDescription] = useState(channel.description ?? "");
  const [avatarFile, setAvatarFile] = useState<File | null>(null);
  const [bannerFile, setBannerFile] = useState<File | null>(null);
  const [fileError, setFileError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const avatarInput = useRef<HTMLInputElement>(null);
  const bannerInput = useRef<HTMLInputElement>(null);

  const pickImage = (f: File | undefined, kind: "avatar" | "banner") => {
    setFileError(null);
    if (!f) return;
    if (!f.type.startsWith("image/")) {
      setFileError("Please choose an image file.");
      return;
    }
    const max = kind === "avatar" ? MAX_AVATAR : MAX_BANNER;
    if (f.size > max) {
      setFileError(`${kind} too large: ${formatBytes(f.size)} (max ${formatBytes(max)})`);
      return;
    }
    if (kind === "avatar") setAvatarFile(f);
    else setBannerFile(f);
  };

  const uploadImage = async (
    file: File,
    purpose: "avatar" | "banner"
  ): Promise<string> => {
    const presign = await apiFetch<{ upload_url: string; object_key: string }>(
      "/api/uploads/presign",
      {
        method: "POST",
        body: JSON.stringify({
          file_name: file.name,
          content_type: file.type,
          size: file.size,
          purpose,
        }),
      }
    );
    await new Promise<void>((resolve, reject) => {
      const xhr = new XMLHttpRequest();
      xhr.open("PUT", presign.upload_url);
      xhr.setRequestHeader("Content-Type", file.type);
      xhr.onload = () => {
        if (xhr.status >= 200 && xhr.status < 300) resolve();
        else reject(new Error(`Upload failed (HTTP ${xhr.status})`));
      };
      xhr.onerror = () => reject(new Error("Upload failed (network error)"));
      xhr.send(file);
    });
    return mediaUrl(presign.object_key);
  };

  const save = async () => {
    if (!name.trim()) {
      setError("Channel name is required.");
      return;
    }
    setSaving(true);
    setError(null);
    try {
      const body: Record<string, unknown> = {
        name: name.trim(),
        description: description.trim() || null,
      };
      if (avatarFile) body.avatarUrl = await uploadImage(avatarFile, "avatar");
      if (bannerFile) body.bannerUrl = await uploadImage(bannerFile, "banner");

      const res = await apiFetch<{ channel: Channel }>(`/api/channels/${channel.id}`, {
        method: "PATCH",
        body: JSON.stringify(body),
      });
      onSaved(res.channel);
      onClose();
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : "Save failed");
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="mt-6 rounded-lg border border-border bg-card p-4 sm:p-6">
      <h2 className="mb-4 text-lg font-semibold">Edit channel</h2>

      <div className="flex flex-col gap-4 sm:flex-row sm:items-start">
        {/* Avatar */}
        <div className="flex flex-col items-center gap-2">
          <Avatar size="lg" className="size-24">
            {avatarFile ? (
              // eslint-disable-next-line @next/next/no-img-element
              <img
                src={URL.createObjectURL(avatarFile)}
                alt="Avatar preview"
                className="size-full rounded-full object-cover"
              />
            ) : channel.avatarUrl ? (
              <AvatarImage src={channel.avatarUrl} alt={channel.name} />
            ) : (
              <AvatarFallback className="text-3xl">
                {channel.name.charAt(0).toUpperCase()}
              </AvatarFallback>
            )}
          </Avatar>
          <Button
            variant="outline"
            size="sm"
            disabled={saving}
            onClick={() => avatarInput.current?.click()}
          >
            {avatarFile ? "Change" : "Upload"} avatar
          </Button>
          <input
            ref={avatarInput}
            type="file"
            accept="image/jpeg,image/png,image/webp"
            className="hidden"
            onChange={(e) => {
              pickImage(e.target.files?.[0], "avatar");
              e.target.value = "";
            }}
          />
          {avatarFile && (
            <p className="text-xs text-muted-foreground">{formatBytes(avatarFile.size)}</p>
          )}
        </div>

        {/* Name + description */}
        <div className="flex flex-1 flex-col gap-4">
          <div>
            <label className="mb-1 block text-sm font-medium">Channel name</label>
            <Input
              value={name}
              onChange={(e) => setName(e.target.value)}
              disabled={saving}
            />
          </div>
          <div>
            <label className="mb-1 block text-sm font-medium">Description</label>
            <textarea
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              disabled={saving}
              rows={3}
              className="w-full rounded-md border border-border bg-transparent px-3 py-2 text-sm placeholder:text-muted-foreground focus:outline-none"
              placeholder="Tell viewers about your channel"
            />
          </div>

          {/* Banner */}
          <div>
            <label className="mb-1 block text-sm font-medium">Banner</label>
            <div className="flex items-center gap-3">
              <div className="h-16 w-48 overflow-hidden rounded-md bg-muted">
                {bannerFile ? (
                  // eslint-disable-next-line @next/next/no-img-element
                  <img
                    src={URL.createObjectURL(bannerFile)}
                    alt="Banner preview"
                    className="h-full w-full object-cover"
                  />
                ) : channel.bannerUrl ? (
                  // eslint-disable-next-line @next/next/no-img-element
                  <img
                    src={channel.bannerUrl}
                    alt="Banner"
                    className="h-full w-full object-cover"
                  />
                ) : (
                  <div className="flex h-full w-full items-center justify-center text-xs text-muted-foreground">
                    No banner
                  </div>
                )}
              </div>
              <Button
                variant="outline"
                size="sm"
                disabled={saving}
                onClick={() => bannerInput.current?.click()}
              >
                {bannerFile ? "Change" : "Upload"} banner
              </Button>
              <input
                ref={bannerInput}
                type="file"
                accept="image/jpeg,image/png,image/webp"
                className="hidden"
                onChange={(e) => {
                  pickImage(e.target.files?.[0], "banner");
                  e.target.value = "";
                }}
              />
            </div>
            {bannerFile && (
              <p className="mt-1 text-xs text-muted-foreground">{formatBytes(bannerFile.size)}</p>
            )}
          </div>
        </div>
      </div>

      {fileError && <p className="mt-3 text-sm text-destructive">{fileError}</p>}
      {error && <p className="mt-3 text-sm text-destructive">{error}</p>}

      <div className="mt-4 flex gap-2">
        <Button onClick={save} disabled={saving}>
          {saving ? "Saving…" : "Save"}
        </Button>
        <Button variant="ghost" onClick={onClose} disabled={saving}>
          Cancel
        </Button>
      </div>
    </div>
  );
}
