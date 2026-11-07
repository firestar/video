/**
 * E2E global setup: seed two users (owner + viewer) and one READY video via
 * the API and a direct Postgres connection.
 *
 * Requires a live stack: API (NEXT_PUBLIC_API_URL) + Postgres (E2E_DATABASE_URL).
 * Exports E2E_STATE so specs can read the seeded ids.
 */
import { Client } from "pg";

const API = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080";
const DB_URL = process.env.E2E_DATABASE_URL ?? "postgres://postgres:postgres@localhost:5432/youtube";
const SUFFIX = Date.now().toString(36);
const OWNER_NAME = `e2e-owner-${SUFFIX}`;
const VIEWER_NAME = `e2e-viewer-${SUFFIX}`;
const OWNER_EMAIL = `${OWNER_NAME}@e2e.test`;
const VIEWER_EMAIL = `${VIEWER_NAME}@e2e.test`;
const PASSWORD = "password123";
const VIDEO_TITLE = `E2E Zebra Flamingo ${SUFFIX}`;
const VIDEO_DESC = "A zebra flamingo dancing. zebrinflamingo";

function register(name: string, email: string) {
  // Argon2 hash is slow (~100ms+); use fetch with a generous timeout.
  return fetch(`${API}/api/auth/register`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      name,
      email,
      password: PASSWORD,
      channelName: name,
    }),
  });
}

async function waitDb() {
  for (let i = 0; i < 30; i++) {
    const c = new Client({ connectionString: DB_URL });
    try {
      await c.connect();
      await c.end();
      return;
    } catch {
      await new Promise((r) => setTimeout(r, 1000));
    }
  }
  throw new Error(`Postgres not reachable at ${DB_URL}`);
}

export default async function globalSetup() {
  // API reachable?
  const health = await fetch(`${API}/api/health`).catch(() => null);
  if (!health || !health.ok) {
    throw new Error(
      `API not reachable at ${API} (is it running? check NEXT_PUBLIC_API_URL)`
    );
  }
  await waitDb();

  const ownerRes = await register(OWNER_NAME, OWNER_EMAIL);
  if (!ownerRes.ok) throw new Error(`register owner failed: ${ownerRes.status} ${await ownerRes.text()}`);
  const owner = (await ownerRes.json()) as { token: string; user: { id: string } };

  const viewerRes = await register(VIEWER_NAME, VIEWER_EMAIL);
  if (!viewerRes.ok) throw new Error(`register viewer failed: ${viewerRes.status} ${await viewerRes.text()}`);
  const viewer = (await viewerRes.json()) as { token: string; user: { id: string } };

  // Presign upload.
  const presignRes = await fetch(`${API}/api/uploads/presign`, {
    method: "POST",
    headers: { "Content-Type": "application/json", Authorization: `Bearer ${owner.token}` },
    body: JSON.stringify({ file_name: "e2e.mp4", content_type: "video/mp4", size: 12345 }),
  });
  if (!presignRes.ok) throw new Error(`presign failed: ${presignRes.status} ${await presignRes.text()}`);
  const presign = (await presignRes.json()) as { upload_url: string; object_key: string };

  // Minimal valid MP4 (empty file would fail ffprobe; but we mark the row
  // ready directly via SQL, so the upload is never transcoded — the file
  // content is irrelevant for the test. Still PUT something to the bucket.
  const putRes = await fetch(presign.upload_url, {
    method: "PUT",
    headers: { "Content-Type": "video/mp4" },
    body: Buffer.from(
      "000000206674797069736f360000000069736f6d69736f32617663316d703431",
      "hex"
    ),
  });
  if (!putRes.ok) throw new Error(`S3 PUT failed: ${putRes.status}`);

  // Create the video row (status=processing).
  const createRes = await fetch(`${API}/api/videos`, {
    method: "POST",
    headers: { "Content-Type": "application/json", Authorization: `Bearer ${owner.token}` },
    body: JSON.stringify({
      title: VIDEO_TITLE,
      description: VIDEO_DESC,
      tags: ["e2e", "zebrinflamingo", "flamingo"],
      object_key: presign.object_key,
    }),
  });
  if (!createRes.ok) throw new Error(`create video failed: ${createRes.status} ${await createRes.text()}`);
  const created = (await createRes.json()) as { video: { id: string; channelId: string } };

  // Mark the video ready directly in the DB.
  const c = new Client({ connectionString: DB_URL });
  await c.connect();
  try {
    const r = await c.query(
      `UPDATE videos SET status = 'ready', duration_secs = 10, file_size = 1024,
       published_at = now()
       WHERE id = $1 RETURNING id, channel_id`,
      [created.video.id]
    );
    if (r.rowCount !== 1) throw new Error(`video update affected 0 rows: ${created.video.id}`);
  } finally {
    await c.end();
  }

  process.env.E2E_OWNER_EMAIL = OWNER_EMAIL;
  process.env.E2E_VIEWER_EMAIL = VIEWER_EMAIL;
  process.env.E2E_OWNER_ID = owner.user.id;
  process.env.E2E_OWNER_TOKEN = owner.token;
  process.env.E2E_OWNER_CHANNEL_ID = created.video.channelId;
  process.env.E2E_VIEWER_ID = viewer.user.id;
  process.env.E2E_VIEWER_TOKEN = viewer.token;
  process.env.E2E_VIDEO_ID = created.video.id;
  process.env.E2E_VIDEO_TITLE = VIDEO_TITLE;

  console.log(
    `E2E seed ok: video=${created.video.id} owner=${owner.user.id} channel=${created.video.channelId}`
  );
}

export function e2eState() {
  return {
    ownerId: process.env.E2E_OWNER_ID as string,
    ownerToken: process.env.E2E_OWNER_TOKEN as string,
    ownerChannelId: process.env.E2E_OWNER_CHANNEL_ID as string,
    viewerId: process.env.E2E_VIEWER_ID as string,
    viewerToken: process.env.E2E_VIEWER_TOKEN as string,
    videoId: process.env.E2E_VIDEO_ID as string,
    videoTitle: process.env.E2E_VIDEO_TITLE as string,
  };
}
