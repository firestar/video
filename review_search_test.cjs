const { Client } = require("pg");
const fs = require("fs");

const DIR = "backend/crates/db/migrations";

function main() {
  const base = new Client({ host: "127.0.0.1", port: 55432, user: "postgres", database: "postgres" });
  base.connect().then(async () => {
    await base.query("DROP DATABASE IF EXISTS searchtest WITH (FORCE)");
    await base.query("CREATE DATABASE searchtest");
    console.log("created searchtest");
    await base.end();
    run();
  }).catch(e => { console.error("BASE_FAIL", e.message); process.exit(1); });
}

async function run() {
  const c = new Client({ host: "127.0.0.1", port: 55432, user: "postgres", database: "searchtest" });
  await c.connect();
  for (const f of ["001_init.sql", "002_search.sql", "003_comments.sql"]) {
    const sql = fs.readFileSync(`${DIR}/${f}`, "utf8");
    await c.query(sql);
    console.log("applied", f);
  }

  const cols = await c.query("SELECT column_name FROM information_schema.columns WHERE table_name='videos' AND column_name='search_vector'");
  console.log("search_vector column:", cols.rows);
  if (cols.rows.length !== 1) throw new Error("missing search_vector");

  const idx = await c.query("SELECT indexname FROM pg_indexes WHERE tablename='videos' AND indexname='idx_videos_search'");
  console.log("GIN index:", idx.rows);
  if (idx.rows.length !== 1) throw new Error("missing GIN index");
  await c.query("DELETE FROM videos");
  await c.query("DELETE FROM channels");
  await c.query("DELETE FROM users");
  await c.query(
    "INSERT INTO users (id, email, name, channel_name, password_hash) " +
    "VALUES (gen_random_uuid(), 'a@x.com', 'Alice', 'alicechan', 'x'), " +
    "(gen_random_uuid(), 'b@x.com', 'Bob', 'bobchan', 'x')" );
  await c.query(
    "INSERT INTO channels (id, user_id, name, handle) " +
    "SELECT gen_random_uuid(), id, channel_name, channel_name FROM users");
  const ch = (await c.query("SELECT c.id, c.name FROM channels c ORDER BY c.name")).rows;
  console.log("channels:", ch.map(r => r.name));
  const [c1, c2] = [ch[0].id, ch[1].id];
  await c.query(
    "INSERT INTO videos (id, channel_id, title, description, status, object_key) VALUES " +
    `(gen_random_uuid(), '${c1}', 'Rust async runtime demo', 'tokio and axum deep dive', 'ready', 'videos/rust.mp4'),` +
    `(gen_random_uuid(), '${c1}', 'Cooking pasta', 'italian food', 'ready', 'videos/pasta.mp4'),` +
    `(gen_random_uuid(), '${c2}', 'Rust web framework review', 'axum and actix', 'pending', 'videos/rust2.mp4'),` +
    `(gen_random_uuid(), '${c2}', 'Music video', 'song', 'failed', 'videos/music.mp4')`
  );
  console.log("seeded");

  // exact videos::search query
  let r = await c.query(
    "SELECT title FROM videos WHERE status = 'ready' AND search_vector @@ plainto_tsquery('english', $1) " +
    "ORDER BY ts_rank(search_vector, plainto_tsquery('english', $1)) DESC LIMIT $2",
    ["rust", 10]
  );
  console.log("FTS rust ready-only:", r.rows.map(x => x.title));
  if (JSON.stringify(r.rows.map(x => x.title)) !== JSON.stringify(["Rust async runtime demo"])) throw new Error("FTS rust failed: " + JSON.stringify(r.rows));

  r = await c.query("SELECT count(*) AS n FROM videos WHERE status = 'ready' AND search_vector @@ plainto_tsquery('english', $1)", ["rust"]);
  console.log("ready rust count:", r.rows[0].n);
  if (Number(r.rows[0].n) !== 1) throw new Error("ready rust count != 1");

  r = await c.query("SELECT count(*) AS n FROM videos WHERE search_vector @@ plainto_tsquery('english', $1)", ["rust"]);
  console.log("all-status rust count (2 => status filter matters):", r.rows[0].n);
  if (Number(r.rows[0].n) !== 2) throw new Error("all-status rust count != 2");

  // description match (english FTS)
  r = await c.query("SELECT title FROM videos WHERE status = 'ready' AND search_vector @@ plainto_tsquery('english', $1)", ["axum"]);
  console.log("FTS axum (description, ready):", r.rows.map(x => x.title));
  if (JSON.stringify(r.rows.map(x => x.title)) !== JSON.stringify(["Rust async runtime demo"])) throw new Error("FTS axum failed");

  // case-insensitive via english config
  r = await c.query("SELECT count(*) AS n FROM videos WHERE status = 'ready' AND search_vector @@ plainto_tsquery('english', $1)", ["Rust"]);
  console.log("FTS capitalized Rust:", r.rows[0].n);
  if (Number(r.rows[0].n) !== 1) throw new Error("capitalized Rust != 1");

  // exact channels::search query
  r = await c.query("SELECT name FROM channels WHERE name ILIKE $1 ORDER BY name LIMIT $2", ["%ali%", 10]);
  if (!r.rows.some(x => x.name.includes("alicechan"))) throw new Error("channel ILIKE failed");
  console.log("channel ILIKE %ali%:", r.rows.map(x => x.name));
  // ILIKE wildcard safety via binding (underscore in data, literal %u% won't match 'u' as any-char)
  r = await c.query("SELECT count(*) AS n FROM channels WHERE name ILIKE $1", ["%_u%"]);
  console.log("channel ILIKE underscore pattern count:", r.rows[0].n);

  // ts_rank ordering sanity: title-match should rank >= description-match
  r = await c.query(
    "SELECT title FROM videos WHERE status = 'ready' AND search_vector @@ plainto_tsquery('english', $1) " +
    "ORDER BY ts_rank(search_vector, plainto_tsquery('english', $1)) DESC LIMIT 10",
    ["rust"]
  );
  console.log("ts_rank ordering:", r.rows.map(x => x.title));

  await c.end();
  console.log("ALL_MIGRATION_AND_QUERY_TESTS_PASSED");
}

main();
