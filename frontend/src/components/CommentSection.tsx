"use client";
import { useCallback, useEffect, useState } from "react";
import { apiFetch } from "@/lib/api";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import { formatCount, formatDate } from "@/lib/format";
import { useAuth } from "@/lib/auth";
import { ThumbsUp, Trash2 } from "lucide-react";
import { CommentSectionSkeleton } from "@/components/Skeletons";
import type { Comment } from "@/lib/types";

interface ThreadedResponse {
  comments: Comment[];
  count: number;
}

function timeAgo(iso: string): string {
  const then = new Date(iso).getTime();
  if (Number.isNaN(then)) return "";
  const secs = Math.max(0, (Date.now() - then) / 1000);
  if (secs < 60) return "just now";
  const mins = Math.floor(secs / 60);
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days < 7) return `${days}d ago`;
  return formatDate(iso);
}

function CommentItem({
  comment,
  videoId,
  isReply,
  onDeleted,
}: {
  comment: Comment;
  videoId: string;
  isReply: boolean;
  onDeleted: (id: string) => void;
}) {
  const { user } = useAuth();
  const [liked, setLiked] = useState(false);
  const [likeCount, setLikeCount] = useState(comment.likeCount ?? 0);
  const [deleting, setDeleting] = useState(false);

  const isAuthor = user?.id === comment.userId;

  const toggleLike = useCallback(async () => {
    if (!user) {
      window.location.href = "/login";
      return;
    }
    try {
      const res = await apiFetch<{ liked: boolean }>(
        `/api/comments/${comment.id}/like`,
        { method: "POST" }
      );
      setLiked(res.liked);
      setLikeCount((c) => (res.liked ? c + 1 : Math.max(0, c - 1)));
    } catch {}
  }, [user, comment.id]);

  const handleDelete = useCallback(async () => {
    if (!user || !confirm("Delete this comment?")) return;
    setDeleting(true);
    try {
      await apiFetch(`/api/comments/${comment.id}`, { method: "DELETE" });
      onDeleted(comment.id);
    } catch (e: unknown) {
      alert(e instanceof Error ? e.message : "Something went wrong");
    } finally {
      setDeleting(false);
    }
  }, [user, comment.id, onDeleted]);

  const name = comment.authorName ?? "Unknown";
  const initial = name.charAt(0).toUpperCase();

  return (
    <div className={`flex gap-3 ${isReply ? "pl-12" : ""}`}>
      <Avatar size="sm">
        {comment.authorAvatar ? (
          <AvatarImage src={comment.authorAvatar} alt={name} />
        ) : (
          <AvatarFallback>{initial}</AvatarFallback>
        )}
      </Avatar>
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium">{name}</span>
          {isAuthor && (
            <span className="rounded bg-muted px-1.5 text-[10px] text-muted-foreground">you</span>
          )}
          <span className="text-xs text-muted-foreground">{timeAgo(comment.createdAt)}</span>
        </div>
        <p className="mt-0.5 whitespace-pre-wrap break-words text-sm text-foreground/80">
          {comment.body}
        </p>
        <div className="mt-1 flex items-center gap-3">
          <button
            onClick={toggleLike}
            className={`flex items-center gap-1 text-xs transition-colors ${
              liked ? "" : "text-muted-foreground hover:text-foreground/80"
            }`}
            aria-pressed={liked}
          >
            <ThumbsUp className="size-3.5" />
            {likeCount > 0 && (
              <span className="tabular-nums">{formatCount(likeCount)}</span>
            )}
          </button>
          {isAuthor && (
            <button
              onClick={handleDelete}
              disabled={deleting}
              className="flex items-center gap-1 text-xs text-muted-foreground hover:text-destructive disabled:opacity-50"
            >
              <Trash2 className="size-3.5" />
              Delete
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

export default function CommentSection({ videoId }: { videoId: string }) {
  const { user } = useAuth();
  const [comments, setComments] = useState<Comment[]>([]);
  const [count, setCount] = useState(0);
  const [loading, setLoading] = useState(true);
  const [body, setBody] = useState("");
  const [replyTo, setReplyTo] = useState<string | null>(null);
  const [replyBody, setReplyBody] = useState("");
  const [expandedReplies, setExpandedReplies] = useState<Set<string>>(new Set());
  const [posting, setPosting] = useState(false);

  const load = useCallback(async () => {
    try {
      const res = await apiFetch<ThreadedResponse>(`/api/comments/${videoId}`);
      setComments(res.comments);
      setCount(res.count);
    } catch {}
    finally {
      setLoading(false);
    }
  }, [videoId]);

  useEffect(() => {
    if (!videoId) return;
    setLoading(true);
    load();
  }, [videoId, load]);

  const post = useCallback(async () => {
    if (!user) {
      window.location.href = "/login";
      return;
    }
    const text = replyTo ? replyBody : body;
    if (!text.trim()) return;
    setPosting(true);
    try {
      await apiFetch(`/api/videos/${videoId}/comments`, {
        method: "POST",
        body: JSON.stringify({
          body: text.trim(),
          parentId: replyTo ?? undefined,
        }),
      });
      setBody("");
      setReplyBody("");
      setReplyTo(null);
      await load();
    } catch (e: unknown) {
      alert(e instanceof Error ? e.message : "Something went wrong");
    } finally {
      setPosting(false);
    }
  }, [user, replyTo, replyBody, body, videoId, load]);

  const handleDeleted = useCallback(
    (id: string) => {
      setComments((prev) =>
        prev
          .map((c) => {
            if (c.id === id) return null;
            return c.replies?.some((r) => r.id === id)
              ? { ...c, replies: c.replies.filter((r) => r.id !== id) }
              : c;
          })
          .filter(Boolean) as Comment[]
      );
      setCount((n) => n - 1);
      if (replyTo === id) {
        setReplyTo(null);
        setReplyBody("");
      }
    },
    [replyTo]
  );

  const toggleReplies = useCallback((id: string) => {
    setExpandedReplies((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }, []);

  if (loading) {
    return <CommentSectionSkeleton count={3} />;
  }

  return (
    <section className="flex flex-col gap-5">
      <h2 className="text-base font-medium">
        {count > 0 ? `${formatCount(count)} comments` : "Comments"}
      </h2>

      {user ? (
        <div className="flex gap-3">
          <Avatar size="sm">
            <AvatarFallback>{(user.name ?? "?").charAt(0).toUpperCase()}</AvatarFallback>
          </Avatar>
          <div className="flex-1">
            <textarea
              value={body}
              onChange={(e) => setBody(e.target.value)}
              placeholder="Add a comment…"
              rows={2}
              className="w-full resize-none rounded-lg bg-muted p-3 text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
            />
            <div className="mt-2 flex justify-end gap-2">
              <Button size="sm" variant="ghost" onClick={() => setBody("")}>
                Cancel
              </Button>
              <Button size="sm" onClick={post} disabled={posting || !body.trim()}>
                {posting ? "Posting…" : "Comment"}
              </Button>
            </div>
          </div>
        </div>
      ) : (
        <div className="rounded-lg bg-muted p-4 text-sm text-muted-foreground">
          <a href="/login" className="text-primary hover:underline">
            Sign in
          </a>{" "}
          to join the conversation.
        </div>
      )}

      {comments.length === 0 ? (
        <p className="text-sm text-muted-foreground">No comments yet.</p>
      ) : (
        <div className="flex flex-col gap-5">
          {comments.map((c) => {
            const replies = c.replies ?? [];
            const expanded = expandedReplies.has(c.id);
            const showAllReplies = expanded || replies.length <= 2;
            return (
              <div key={c.id} className="flex flex-col gap-3">
                <CommentItem
                  comment={c}
                  videoId={videoId}
                  isReply={false}
                  onDeleted={handleDeleted}
                />
                <div className="ml-9 flex items-center gap-3">
                  {user && (
                    <button
                      onClick={() => {
                        if (replyTo === c.id) {
                          setReplyTo(null);
                          setReplyBody("");
                        } else {
                          setReplyTo(c.id);
                          setReplyBody("");
                        }
                      }}
                      className="text-xs text-muted-foreground hover:text-foreground/80"
                    >
                      Reply
                    </button>
                  )}
                  {replies.length > 2 && (
                    <button
                      onClick={() => toggleReplies(c.id)}
                      className="text-xs text-muted-foreground hover:text-foreground/80"
                    >
                      {expanded ? "Hide replies" : `${replies.length} replies`}
                    </button>
                  )}
                </div>
                {replyTo === c.id && user && (
                  <div className="ml-9 flex gap-3">
                    <Avatar size="sm">
                      <AvatarFallback>
                        {(user.name ?? "?").charAt(0).toUpperCase()}
                      </AvatarFallback>
                    </Avatar>
                    <div className="flex-1">
                      <textarea
                        value={replyBody}
                        onChange={(e) => setReplyBody(e.target.value)}
                        placeholder={`Reply to ${c.authorName ?? "comment"}…`}
                        rows={2}
                        autoFocus
                        className="w-full resize-none rounded-lg bg-muted p-3 text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
                      />
                      <div className="mt-2 flex justify-end gap-2">
                        <Button
                          size="sm"
                          variant="ghost"
                          onClick={() => {
                            setReplyTo(null);
                            setReplyBody("");
                          }}
                        >
                          Cancel
                        </Button>
                        <Button
                          size="sm"
                          onClick={post}
                          disabled={posting || !replyBody.trim()}
                        >
                          {posting ? "Posting…" : "Reply"}
                        </Button>
                      </div>
                    </div>
                  </div>
                )}
                {showAllReplies &&
                  replies.map((r) => (
                    <CommentItem
                      key={r.id}
                      comment={r}
                      videoId={videoId}
                      isReply={true}
                      onDeleted={handleDeleted}
                    />
                  ))}
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
}
