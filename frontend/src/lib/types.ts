export interface User {
  id: string;
  name: string;
  email: string;
  avatar_url?: string;
}

export interface Channel {
  id: string;
  user_id: string;
  name: string;
  handle: string;
  description?: string;
  banner_url?: string;
  avatar_url?: string;
  subscriber_count?: number;
  created_at: string;
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
}

export interface Comment {
  id: string;
  video_id: string;
  user_id: string;
  user?: User;
  body: string;
  created_at: string;
}
