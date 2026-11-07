/// E2E shared state, exported from e2e/global-setup.ts.
export interface E2EState {
  ownerId: string;
  ownerToken: string;
  ownerChannelId: string;
  viewerId: string;
  viewerToken: string;
  videoId: string;
  videoTitle: string;
}
