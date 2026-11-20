import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import { validateManifest } from "./manifest.mjs";
import { FILES, pack } from "./pack.mjs";
import { createZip, listZip } from "./zip.mjs";

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

test("the real plugin.json is valid (provider.wasm is not required here)", () => {
  const manifest = JSON.parse(fs.readFileSync(path.join(repo, "plugin.json"), "utf8"));
  assert.deepEqual(validateManifest(manifest), []);
  assert.equal(manifest.id, "bsmanager");
  assert.equal(manifest.apiVersion, 1);
  assert.deepEqual(manifest.permissions.http, []);
  assert.equal(manifest.permissions.read, undefined);
  assert.deepEqual(manifest.permissions.folders.map((f) => [f.key, f.as]), [
    ["instances", "/instances"],
    ["app", "/app"],
  ]);
});

test("validateManifest checks permissions.folders like the host", () => {
  const base = {
    id: "my-store",
    name: "X",
    version: "1.0.0",
    apiVersion: 1,
    providers: [{ wasm: "provider.wasm" }],
  };
  const folder = { key: "games", label: "Games", as: "/games", default: "~/Games" };
  const check = (folders, read = []) =>
    validateManifest({ ...base, permissions: { read, folders } });
  assert.deepEqual(check([folder]), []);
  assert.ok(check([{ ...folder, key: "Games" }]).length > 0);
  assert.ok(check([{ ...folder, label: " " }]).length > 0);
  assert.ok(check([{ ...folder, default: "" }]).length > 0);
  assert.ok(check([{ ...folder, as: "/data" }]).length > 0);
  assert.ok(check([folder, { ...folder, as: "/other" }]).length > 0);
  assert.ok(check([folder], [{ path: "~/X", as: "/games" }]).length > 0);
});

test("validateManifest follows the host's rules", () => {
  const ok = {
    id: "my-store",
    name: "X",
    version: "1.0.0",
    apiVersion: 1,
    providers: [{ wasm: "provider.wasm" }],
    permissions: { read: [{ path: "~/X", as: "/x" }], http: ["*.example.com", "http://localhost"] },
  };
  assert.deepEqual(validateManifest(ok), []);
  const bad = (change) => validateManifest({ ...ok, ...change }).length > 0;
  assert.ok(bad({ id: "A" }));
  assert.ok(bad({ version: "1.0" }));
  assert.ok(bad({ apiVersion: 2 }));
  assert.ok(bad({ providers: [{ wasm: "../x.wasm" }] }));
  assert.ok(bad({ providers: [{ wasm: "C:/x.wasm" }] }));
  assert.ok(bad({ permissions: { read: [{ path: "a", as: "/data" }] } }));
  assert.ok(bad({ permissions: { read: [{ path: "a", as: "x" }] } }));
  assert.ok(bad({ permissions: { read: [{ path: "a", as: "/x" }, { path: "b", as: "/x" }] } }));
  assert.ok(bad({ permissions: { http: ["not a host"] } }));
  assert.ok(validateManifest(ok, () => false).length > 0);
});

test("pack writes a zip with the plugin files at the top level", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "bsmanager-pack-"));
  try {
    for (const name of FILES) fs.writeFileSync(path.join(root, name), `content of ${name}`);
    fs.copyFileSync(path.join(repo, "plugin.json"), path.join(root, "plugin.json"));
    const result = pack(root);
    assert.equal(result.name, `bsmanager-${JSON.parse(fs.readFileSync(path.join(root, "plugin.json"), "utf8")).version}.zip`);
    assert.match(result.sha256, /^[0-9a-f]{64}$/);
    const zip = fs.readFileSync(result.file);
    assert.equal(zip.length, result.size);
    assert.deepEqual(listZip(zip), FILES);
    assert.ok(listZip(zip).every((n) => !n.includes("\\") && !n.includes("/")));
    fs.rmSync(path.join(root, "provider.wasm"));
    assert.throws(() => pack(root), /provider\.wasm/);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test("createZip is deterministic and rejects backslash names", () => {
  const entries = [{ name: "a.txt", data: Buffer.from("hello") }];
  assert.deepEqual(createZip(entries), createZip(entries));
  assert.throws(() => createZip([{ name: "a\\b.txt", data: Buffer.alloc(0) }]));
});
