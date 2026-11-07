import { defineConfig, devices } from "@playwright/test";

// E2E runs against a live stack:
//   - API:   http://localhost:8080 (postgres:5432 + redis:6379 + storage)
//   - Web:   http://localhost:3000 (next dev)
// Bring them up first (docker compose up / dev scripts), then:
//   npm run test:e2e

const BASE_URL = process.env.E2E_BASE_URL ?? "http://localhost:3000";
const API_URL = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080";
const DB_URL = process.env.E2E_DATABASE_URL ?? "postgres://postgres:postgres@localhost:5432/youtube";

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: false,
  workers: 1,
  forbidOnly: !!process.env.CI,
  retries: 0,
  timeout: 120_000,
  expect: { timeout: 20_000 },
  reporter: process.env.CI ? "github" : "list",
  use: {
    baseURL: BASE_URL,
    trace: "on-first-retry",
    video: "off",
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
  // Seeds test users + a ready video (global-setup.ts), using the API and
  // a direct Postgres connection for the final "ready" state.
  globalSetup: "./e2e/global-setup.ts",
});
