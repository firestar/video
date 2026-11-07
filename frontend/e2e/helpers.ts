import type { Page } from "@playwright/test";

/** Seeded credentials from global-setup.ts env vars. */
export const seed = {
  get ownerId() {
    return process.env.E2E_OWNER_ID as string;
  },
  get ownerToken() {
    return process.env.E2E_OWNER_TOKEN as string;
  },
  get ownerEmail() {
    return process.env.E2E_OWNER_EMAIL as string;
  },
  get ownerChannelId() {
    return process.env.E2E_OWNER_CHANNEL_ID as string;
  },
  get viewerId() {
    return process.env.E2E_VIEWER_ID as string;
  },
  get viewerToken() {
    return process.env.E2E_VIEWER_TOKEN as string;
  },
  get viewerEmail() {
    return process.env.E2E_VIEWER_EMAIL as string;
  },
  get videoId() {
    return process.env.E2E_VIDEO_ID as string;
  },
  get videoTitle() {
    return process.env.E2E_VIDEO_TITLE as string;
  },
} as const;

/**
 * Log a seeded user in through the /login page.
 * Both seeded users share password `password123`.
 * Waits for navigation away from /login.
 */
export async function loginAs(page: Page, which: "owner" | "viewer") {
  const email = which === "owner" ? seed.ownerEmail : seed.viewerEmail;
  await page.goto("/login");
  await page.getByPlaceholder("Email").fill(email);
  await page.getByPlaceholder("Password").fill("password123");
  await page.getByRole("button", { name: "Login" }).click();
  await page.waitForURL((url) => !url.pathname.startsWith("/login"));
}
