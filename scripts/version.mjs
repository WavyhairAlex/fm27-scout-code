#!/usr/bin/env node

/**
 * Single source of truth for the app version.
 *
 *   node scripts/version.mjs              # print the current version
 *   node scripts/version.mjs 1.2.0        # bump Cargo.toml, package.json and Cargo.lock
 *   node scripts/version.mjs --check      # verify every tracked copy matches
 *
 * The root Cargo.toml [workspace.package].version is canonical. Tauri reads it
 * directly because tauri.conf.json no longer sets a version, and the frontend
 * receives it through Vite's __APP_VERSION__ define.
 */

import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const cargoPath = resolve(root, "Cargo.toml");
const packagePath = resolve(root, "apps/desktop/package.json");
const tauriPath = resolve(root, "apps/desktop/src-tauri/tauri.conf.json");

const CARGO_VERSION_RE =
  /(^\[workspace\.package\][^[]*?^version\s*=\s*")([^"]+)(")/m;
const SEMVER_RE = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/;

function fail(message) {
  console.error(`error: ${message}`);
  process.exit(1);
}

function readCargoVersion() {
  const match = CARGO_VERSION_RE.exec(readFileSync(cargoPath, "utf8"));
  if (!match) {
    fail("could not find [workspace.package].version in Cargo.toml");
  }
  return match[2];
}

function writeCargoVersion(version) {
  const text = readFileSync(cargoPath, "utf8");
  if (!CARGO_VERSION_RE.test(text)) {
    fail("could not find [workspace.package].version in Cargo.toml");
  }
  const next = text.replace(CARGO_VERSION_RE, (_match, prefix, _old, suffix) => {
    return `${prefix}${version}${suffix}`;
  });
  writeFileSync(cargoPath, next);
}

function readPackageVersion() {
  return JSON.parse(readFileSync(packagePath, "utf8")).version;
}

function writePackageVersion(version) {
  const config = JSON.parse(readFileSync(packagePath, "utf8"));
  config.version = version;
  writeFileSync(packagePath, `${JSON.stringify(config, null, 2)}\n`);
}

function readTauriVersion() {
  const config = JSON.parse(readFileSync(tauriPath, "utf8"));
  return typeof config.version === "string" ? config.version : null;
}

function writeTauriVersion(version) {
  const config = JSON.parse(readFileSync(tauriPath, "utf8"));
  // Tauri inherits Cargo.toml when version is omitted; only update an explicit
  // value so the script also works with older checkouts.
  if (!Object.hasOwn(config, "version")) return;
  config.version = version;
  writeFileSync(tauriPath, `${JSON.stringify(config, null, 2)}\n`);
}

function check() {
  const cargoVersion = readCargoVersion();
  const packageVersion = readPackageVersion();
  const tauriVersion = readTauriVersion();
  const errors = [];

  if (packageVersion !== cargoVersion) {
    errors.push(
      `apps/desktop/package.json is ${packageVersion}, expected ${cargoVersion}`,
    );
  }
  if (tauriVersion !== null && tauriVersion !== cargoVersion) {
    errors.push(
      `apps/desktop/src-tauri/tauri.conf.json is ${tauriVersion}, expected ${cargoVersion}`,
    );
  }

  if (errors.length > 0) {
    for (const error of errors) console.error(`error: ${error}`);
    console.error("Run `node scripts/version.mjs <version>` to resync.");
    process.exit(1);
  }

  console.log(`version ok: ${cargoVersion}`);
}

function bump(version) {
  if (!SEMVER_RE.test(version)) {
    fail(`'${version}' is not a valid semver version (expected x.y.z)`);
  }

  const current = readCargoVersion();
  if (current === version) {
    console.log(`already at ${version}`);
    return;
  }

  writeCargoVersion(version);
  writePackageVersion(version);
  writeTauriVersion(version);

  try {
    execFileSync("cargo", ["update", "--workspace"], {
      cwd: root,
      stdio: "inherit",
    });
  } catch {
    fail(
      "failed to update Cargo.lock; ensure `cargo` is on PATH, then run `cargo update --workspace`",
    );
  }

  console.log(`bumped ${current} -> ${version}`);
  console.log(
    [
      "next:",
      `  git add Cargo.toml Cargo.lock apps/desktop/package.json apps/desktop/src-tauri/tauri.conf.json`,
      `  git commit -m "chore: release v${version}"`,
      `  git tag v${version}`,
      "  git push --follow-tags",
    ].join("\n"),
  );
}

const args = process.argv.slice(2);

if (args.length === 0) {
  console.log(readCargoVersion());
} else if (args[0] === "--check") {
  check();
} else if (args[0] === "--help" || args[0] === "-h") {
  console.log("usage: node scripts/version.mjs [<version>|--check]");
} else if (args.length === 1) {
  bump(args[0]);
} else {
  fail("expected a single version argument, --check, or no arguments");
}
