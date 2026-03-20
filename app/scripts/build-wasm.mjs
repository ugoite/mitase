import { spawnSync } from "node:child_process";
import { existsSync, rmSync } from "node:fs";
import path from "node:path";

const home = process.env.HOME ?? process.env.USERPROFILE ?? "";
const repoRoot = path.resolve(process.cwd(), "..");
const repoCargoHome = path.join(repoRoot, ".tooling", "cargo-home");
const cargoHome = process.env.CARGO_HOME ?? path.join(home, ".cargo");
const candidates = [
  process.env.WASM_PACK,
  path.join(repoCargoHome, "bin", "wasm-pack"),
  "wasm-pack",
  path.join(cargoHome, "bin", "wasm-pack"),
].filter(Boolean);
const repoCargoBin = path.join(repoCargoHome, "bin");

for (const candidate of candidates) {
  const looksLikePath = candidate.includes(path.sep);
  if (looksLikePath && !existsSync(candidate)) {
    continue;
  }

  const result = spawnSync(
    candidate,
    ["build", "wasm", "--target", "web", "--out-dir", "../src/wasm", "--out-name", "syu_app_wasm"],
    {
      cwd: process.cwd(),
      env: {
        ...process.env,
        PATH: [existsSync(repoCargoBin) ? repoCargoBin : null, process.env.PATH]
          .filter(Boolean)
          .join(path.delimiter),
      },
      stdio: "inherit",
      shell: process.platform === "win32" && candidate === "wasm-pack",
    },
  );

  if (!result.error) {
    if ((result.status ?? 0) === 0) {
      rmSync(path.join(process.cwd(), "wasm", "target"), {
        force: true,
        recursive: true,
      });
    }
    process.exit(result.status ?? 0);
  }
}

console.error(
  "Unable to locate `wasm-pack`. Set WASM_PACK or install it via `cargo install wasm-pack --locked`.",
);
process.exit(1);
