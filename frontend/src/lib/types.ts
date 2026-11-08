export interface User {
  id: string;
  name: string;
  email: string;
  avatar_url?: string;
}

export interface Channel {
  id: string;
  userId: string;
  name: string;
  handle: string;
  description?: string;
  bannerUrl?: string;
  avatarUrl?: string;
  createdAt: string;
}

export interface Video {
  id: string;
  channelId: string;
  title: string;
  description?: string;
  thumbnailUrl?: string;
  objectKey: string;
  tags: string[];
  status: "pending" | "processing" | "ready" | "failed";
  durationSecs?: number;
  fileSize?: number;
  viewCount: number;
  likeCount: number;
  publishedAt?: string;
  createdAt: string;
  /** Joined by GET /api/videos (list) — may be null. */
  channel?: Channel;
  /** Feed reason badge (from /api/home/feed): "trending" | "subscription" | "related". */
  reason?: string;
}

export interface Comment {
  id: string;
  videoId: string;
  userId: string;
  parentId: string | null;
  body: string;
  likeCount: number;
  createdAt: string;
  /** Author name, batch-joined by GET /api/comments/{videoId}. */
  authorName?: string;
  /** Author avatar URL (may be null). */
  authorAvatar?: string | null;
  /** Replies, only on top-level comments. */
  replies?: Comment[];
}

/** A single watch-history row returned by GET /api/history. */
export interface HistoryItem {
  userId: string;
  videoId: string;
  watchedUntilSecs: number;
  updatedAt: string;
  /** Joined video details from GET /api/history. */
  title?: string;
  thumbnailUrl?: string;
  viewCount?: number;
  channel?: Channel;
}

/** GET /api/home/feed response. */
export interface FeedResponse {
  videos: Video[];
  reasons: Record<string, string>;
}

/** A user-owned playlist. */
export interface Playlist {
  id: string;
  userId: string;
  title: string;
  description: string;
  isPublic: boolean;
  createdAt: string;
  updatedAt: string;
  videoCount: number;
}

/** GET /api/playlists/{id} response. */
export interface PlaylistDetail {
  playlist: Playlist;
  videos: Video[];
}
