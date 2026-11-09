"use client";
import { useState } from "react";
import { apiFetch } from "@/lib/api";
import { Button } from "@/components/ui/button";

const REASONS = [
  "Spam or deceptive",
  "Harassment or bullying",
  "Inappropriate content",
  "Hate speech",
  "Copyright",
  "Other",
];

interface Props {
  targetType: "video" | "comment";
  targetId: string;
  title: string;
  onClose: () => void;
}

export default function ReportDialog({ targetType, targetId, title, onClose }: Props) {
  const [reason, setReason] = useState("");
  const [detail, setDetail] = useState("");
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!reason) {
      setError("Please choose a reason.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await apiFetch("/api/reports", {
        method: "POST",
        body: JSON.stringify({ targetType, targetId, reason, detail: detail || undefined }),
      });
      setDone(true);
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
      onClick={onClose}
    >
      <div
        className="w-full max-w-md rounded-lg border border-border bg-background p-5"
        onClick={(e) => e.stopPropagation()}
      >
        {done ? (
          <div className="space-y-4">
            <h2 className="text-base font-semibold">Report submitted</h2>
            <p className="text-sm text-muted-foreground">
              Thanks for letting us know. Our team will review this {targetType}.
            </p>
            <div className="flex justify-end">
              <Button onClick={onClose}>Close</Button>
            </div>
          </div>
        ) : (
          <form onSubmit={submit} className="space-y-4">
            <h2 className="text-base font-semibold">Report {title}</h2>
            {error && (
              <p className="rounded-md border border-red-500/40 bg-red-500/10 px-3 py-2 text-sm text-red-600 dark:text-red-400">
                {error}
              </p>
            )}
            <div>
              <label className="text-xs font-medium text-muted-foreground">Reason</label>
              <div className="mt-1 flex flex-col gap-1">
                {REASONS.map((r) => (
                  <label
                    key={r}
                    className="flex cursor-pointer items-center gap-2 rounded-md px-2 py-1.5 text-sm hover:bg-muted"
                  >
                    <input
                      type="radio"
                      name="reason"
                      value={r}
                      checked={reason === r}
                      onChange={() => setReason(r)}
                    />
                    {r}
                  </label>
                ))}
              </div>
            </div>
            <div>
              <label className="text-xs font-medium text-muted-foreground">
                Additional details (optional)
              </label>
              <textarea
                className="mt-1 w-full rounded-md border border-border bg-background px-3 py-2 text-sm outline-none focus:border-ring focus:ring-3 focus:ring-ring/30"
                rows={3}
                value={detail}
                onChange={(e) => setDetail(e.target.value)}
                placeholder="Tell us more about what happened"
              />
            </div>
            <div className="flex justify-end gap-2">
              <Button type="button" variant="outline" onClick={onClose}>
                Cancel
              </Button>
              <Button type="submit" disabled={busy}>
                {busy ? "Submitting…" : "Submit report"}
              </Button>
            </div>
          </form>
        )}
      </div>
    </div>
  );
}
