const { Client } = require("pg");

async function run() {
  const c = new Client({ host: "127.0.0.1", port: 5432, user: "postgres", database: "youtube" });
  await c.connect();
  // Find or create a channel
  let ch = (await c.query("SELECT id, name FROM channels LIMIT 1")).rows;
  let channelId;
  if (ch.length === 0) {
    const u = (await c.query(
      "INSERT INTO users (email, name, channel_name, password_hash) VALUES ('review@x.com', 'Rev', 'revchan', 'x') RETURNING id"
    )).rows[0];
    channelId = (await c.query(
      "INSERT INTO channels (user_id, name, handle) VALUES ($1, 'RevChannel', 'revhandle') RETURNING id",
      [u.id]
    )).rows[0].id;
  } else {
    channelId = ch[0].id;
  }
  // Insert a ready video with a distinctive searchable title
  await c.query(
    "INSERT INTO videos (channel_id, title, description, status, object_key) VALUES ($1, 'Zqxwv Rust Programming Deep Dive', 'tokio axum sqlx coverage', 'ready', 'videos/zqxwv.mp4')",
    [channelId]
  );
  const id = (await c.query("SELECT id FROM videos WHERE title LIKE 'Zqxwv%'")).rows[0].id;
  console.log("SEEDED_VIDEO_ID=" + id);
  await c.end();
}
run().catch(e => { console.error("FAIL", e.message); process.exit(1); });
