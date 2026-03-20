import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests",
  fullyParallel: true,
  use: {
    baseURL: "http://127.0.0.1:3000",
    headless: true,
  },
  webServer: {
    command: "cargo run -- app . --bind 127.0.0.1 --port 3000",
    cwd: "..",
    url: "http://127.0.0.1:3000/healthz",
    reuseExistingServer: true,
    timeout: 120000,
  },
});
