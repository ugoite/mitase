import { defineConfig } from "@playwright/test";

const appHost = "127.0.0.1";
const appPort = process.env.SYU_APP_E2E_PORT ?? "3000";
const appBaseUrl = `http://${appHost}:${appPort}`;

export default defineConfig({
  testDir: "./tests",
  fullyParallel: true,
  use: {
    baseURL: appBaseUrl,
    headless: true,
  },
  webServer: {
    command: `cargo run -- app . --bind ${appHost} --port ${appPort}`,
    cwd: "..",
    url: `${appBaseUrl}/healthz`,
    reuseExistingServer: true,
    timeout: 120000,
  },
});
