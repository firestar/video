import { test, expect } from "@playwright/test";
import { seed, loginAs } from "./helpers";

test("home page shows the seeded video title", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByText(seed.videoTitle)).toBeVisible();
});

test("search shows the seeded video title", async ({ page }) => {
  await page.goto("/search?q=zebrinflamingo");
  await expect(page.getByText(seed.videoTitle)).toBeVisible();
});

test("watch page shows the video title", async ({ page }) => {
  await page.goto(`/watch/${seed.videoId}`);
  await expect(page.getByText(seed.videoTitle)).toBeVisible();
});

test("channel page loads with a heading", async ({ page }) => {
  await page.goto(`/channel/${seed.ownerChannelId}`);
  await expect(page.locator("h1").first()).toBeVisible();
});

test("logged-in viewer sees a Subscribe button on a watch page", async ({ page }) => {
  await loginAs(page, "viewer");
  await page.goto(`/watch/${seed.videoId}`);
  await expect(page.getByRole("button", { name: /subscribe/i })).toBeVisible();
});
