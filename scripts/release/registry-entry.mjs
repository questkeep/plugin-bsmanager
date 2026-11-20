#!/usr/bin/env node
// Writes the plugin registry's `versions.json` entry of a release (docs/PLUGIN_REGISTRY.md in the
// Questkeep repository) as JSON, ready to paste.
//
//   node scripts/release/registry-entry.mjs --version 0.1.0 --repo questkeep/plugin-bsmanager \
//     --file bsmanager-0.1.0.zip --sha256 <hex> --size <bytes> --notes notes.md \
//     [--min-app 0.6.0] [--released 2026-11-20] [--out entry.json]
import fs from "node:fs";
import { buildRegistryEntry } from "./lib.mjs";

const args = process.argv.slice(2);
function option(name, fallback) {
  const i = args.indexOf(`--${name}`);
  const value = i < 0 ? undefined : args[i + 1];
  if (value === undefined || value.startsWith("--")) {
    if (fallback !== undefined) return fallback;
    console.error(`error: --${name} is required`);
    process.exit(1);
  }
  return value;
}

const entry = buildRegistryEntry({
  version: option("version"),
  repo: option("repo"),
  file: option("file"),
  sha256: option("sha256"),
  size: Number(option("size")),
  minApp: option("min-app", "0.6.0"),
  released: option("released", new Date().toISOString().slice(0, 10)),
  notes: fs.readFileSync(option("notes"), "utf8"),
});
const json = `${JSON.stringify(entry, null, 2)}\n`;
const out = args.includes("--out") ? option("out") : undefined;
if (out) fs.writeFileSync(out, json);
else process.stdout.write(json);
