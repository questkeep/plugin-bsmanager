#!/usr/bin/env node
// Packs the plugin into dist/bsmanager-<version>.zip: plugin.json, provider.wasm, icon.svg,
// README.md and LICENSE at the top level of the zip. Run the build first (pnpm/npm run build).
//
//   node scripts/pack.mjs
//
// Prints `file=`, `sha256=` and `size=` lines (also appended to $GITHUB_OUTPUT when it is set).
import { createHash } from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { validateManifest } from "./manifest.mjs";
import { createZip } from "./zip.mjs";

export const FILES = ["plugin.json", "provider.wasm", "icon.svg", "README.md", "LICENSE"];

/** Packs the plugin in `root` into `<root>/dist` and returns `{ file, name, sha256, size }`. */
export function pack(root) {
  const manifest = JSON.parse(fs.readFileSync(path.join(root, "plugin.json"), "utf8"));
  const problems = validateManifest(manifest, (rel) => fs.existsSync(path.join(root, rel)));
  if (problems.length > 0) throw new Error(`plugin.json is not valid:\n- ${problems.join("\n- ")}`);
  for (const f of FILES) {
    if (!fs.existsSync(path.join(root, f))) throw new Error(`${f} is missing`);
  }
  const zip = createZip(FILES.map((name) => ({ name, data: fs.readFileSync(path.join(root, name)) })));
  const distDir = path.join(root, "dist");
  fs.mkdirSync(distDir, { recursive: true });
  const name = `${manifest.id}-${manifest.version}.zip`;
  const file = path.join(distDir, name);
  fs.writeFileSync(file, zip);
  return {
    file,
    name,
    sha256: createHash("sha256").update(zip).digest("hex"),
    size: zip.length,
  };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  try {
    const result = pack(root);
    const lines = [`file=${result.file}`, `sha256=${result.sha256}`, `size=${result.size}`];
    console.log(lines.join("\n"));
    if (process.env.GITHUB_OUTPUT) fs.appendFileSync(process.env.GITHUB_OUTPUT, `${lines.join("\n")}\n`);
  } catch (error) {
    console.error(`error: ${error.message}`);
    process.exit(1);
  }
}
