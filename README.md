# YouTube Clone

A YouTube-style video platform: Rust backend API + FFmpeg processing worker,
Next.js frontend, Postgres, Redis, and S3-compatible object storage.

## Overview

| Component    | Stack                                              |
|--------------|----------------------------------------------------|
| Backend API  | Rust — Axum 0.8, SQLx, Tokio                       |
| Worker       | Rust — consumes transcode jobs from Redis, FFmpeg  |
| Frontend     | Next.js 15 (App Router, TypeScript, Tailwind, React Query) |
| Primary DB   | Postgres 16                                        |
| Cache/Queue  | Redis 7                                            |
| Storage      | S3-compatible (MinIO / Garage) — bucket `media`    |

Auth uses JWT Bearer tokens checked inline in handlers (no Axum middleware).

## Architecture

```mermaid
flowchart LR
    Browser[Browser] --> FE[Next.js frontend :3000]
    FE --> API[Backend API :8080]
    API --> PG[(Postgres)]
    API --> RD[(Redis)]
    API --> S3[(S3 storage bucket: media)]
    API -- publish transcode job --> RD
    Worker[FFmpeg worker] -- consume transcode job --> RD
    Worker --> S3
    Worker --> PG
```

Upload flow: client → API → S3; API enqueues a transcode job on Redis; the
worker drains the queue, runs FFmpeg (thumbnail, duration, transcode), and
writes outputs back to S3.

## Prerequisites

- Rust (stable, `cargo`)
- Node 20+
- Docker + Docker Compose

## Quick Start

1. Start infra (Postgres, Redis) and services:

   ```bash
   docker compose up -d
   ```

   Or, if running locally instead of via compose:

   ```bash
   cd backend && cargo run          # backend API on :8080
   cd frontend && npm ci && npm run dev   # frontend on :3000
   ```

2. The API listens on `http://localhost:8080`; the frontend on
   `http://localhost:3000`.

## Configuration

Environment variables (all prefixed `YT_`), as declared in `docker-compose.yml`:

| Variable            | Purpose                                        |
|---------------------|------------------------------------------------|
| `YT_DATABASE_URL`   | Postgres connection string                     |
| `YT_REDIS_URL`      | Redis connection string                        |
| `YT_JWT_SECRET`     | Secret for signing/verifying JWT auth tokens   |
| `YT_MINIO_*`        | S3-compatible storage (endpoint, bucket, credentials) |

In `docker-compose.yml` the API and worker services receive:

- `YT_DATABASE_URL=postgres://postgres:postgres@postgres:5432/youtube`
- `YT_REDIS_URL=redis://redis:6379`

Plus whatever `YT_*` vars you add via `.env`.

## Testing

Backend:

```bash
cd backend && cargo test
```

Frontend:

```bash
cd frontend && npm run lint && npm run build
```

End-to-end: start the full stack, then

```bash
cd frontend && npm run test:e2e
```

E2E environment variables:

- `E2E_BASE_URL` — base URL of the running frontend
- `NEXT_PUBLIC_API_URL` — base URL of the running backend API
- `E2E_DATABASE_URL` — Postgres connection string the tests assert against

## Deployment

Deploys to Kaiad (panel: `https://panel.kaiad.dev`, `docker-backend` agent,
`production` env). Service definitions live in `kaiad.yaml`:

- `video-backend` — API service
- `video-worker` — FFmpeg transcode worker
- `video-frontend` — Next.js frontend

Storage backend is Garage (S3-compatible), bucket `media`, endpoint
`https://video.synload.com`.
