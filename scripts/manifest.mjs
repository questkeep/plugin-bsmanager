// Checks a plugin.json against the rules of Questkeep's plugin host
// (crates/questkeep-plugins/src/manifest.rs in the app repository, `validate`). Pure: no I/O.

export const API_VERSION = 1;

const isPlainRelative = (p) =>
  typeof p === "string" &&
  p.trim() !== "" &&
  !p.includes(":") &&
  !/^[/\\]/.test(p) &&
  !p.split(/[/\\]/).some((part) => part === "..");

function checkMount(mount) {
  const parts = String(mount).split("/");
  const plain =
    String(mount).startsWith("/") &&
    parts.length >= 2 &&
    parts.slice(1).every((p) => p !== "" && p !== "." && p !== ".." && !/[\\:\u0000-\u001f]/.test(p));
  if (!plain) return `The folder name "${mount}" is not valid: use an absolute path such as /store.`;
  if (["/plugin", "/data"].some((r) => mount === r || mount.startsWith(`${r}/`))) {
    return `The folder name "${mount}" is taken: /plugin and /data belong to the plugin itself.`;
  }
  return null;
}

/**
 * Returns a list of problems (empty when the manifest is valid). `fileExists(relativePath)` is
 * asked for every provider file.
 */
export function validateManifest(manifest, fileExists = () => true) {
  const problems = [];
  const add = (message) => problems.push(message);
  if (typeof manifest !== "object" || manifest === null) return ["plugin.json is not an object"];

  if (!/^[a-z0-9][a-z0-9-]{1,63}$/.test(manifest.id ?? "")) {
    add(`The plugin id "${manifest.id}" is not valid: use 2 to 64 lowercase letters, digits and dashes.`);
  }
  if (typeof manifest.name !== "string" || manifest.name.trim() === "") add("The plugin has no name.");
  if (typeof manifest.version !== "string" || !/^\d+\.\d+\.\d+$/.test(manifest.version)) {
    add(`The version "${manifest.version}" is not X.Y.Z (the registry needs semver).`);
  }
  if (manifest.apiVersion !== API_VERSION) {
    add(`apiVersion must be ${API_VERSION} (got ${manifest.apiVersion}).`);
  }
  const providers = manifest.providers ?? [];
  if (!Array.isArray(providers) || providers.length === 0) add("The plugin lists no provider.");
  for (const provider of Array.isArray(providers) ? providers : []) {
    if (!isPlainRelative(provider?.wasm)) {
      add(`The store file "${provider?.wasm}" must be a path inside the plugin folder.`);
    } else if (!fileExists(provider.wasm)) {
      add(`The store file "${provider.wasm}" does not exist (run the build first).`);
    }
  }
  const mounts = [];
  for (const read of manifest.permissions?.read ?? []) {
    if (typeof read?.path !== "string" || read.path.trim() === "") {
      add("A folder permission has no path.");
      continue;
    }
    const problem = checkMount(read.as);
    if (problem) add(problem);
    if (mounts.includes(read.as)) add(`Two folder permissions use the same name "${read.as}".`);
    mounts.push(read.as);
  }
  const keys = [];
  for (const folder of manifest.permissions?.folders ?? []) {
    if (!/^[a-z0-9-]{1,32}$/.test(folder?.key ?? "")) {
      add(`The folder key "${folder?.key}" is not valid: use 1 to 32 lowercase letters, digits and dashes.`);
    }
    if (keys.includes(folder?.key)) add(`Two folders use the same key "${folder.key}".`);
    keys.push(folder?.key);
    if (typeof folder?.label !== "string" || folder.label.trim() === "") {
      add(`The folder "${folder?.key}" has no label.`);
    }
    if (folder?.default !== undefined && (typeof folder.default !== "string" || folder.default.trim() === "")) {
      add(`The default of the folder "${folder?.key}" is empty.`);
    }
    const problem = checkMount(folder?.as);
    if (problem) add(problem);
    if (mounts.includes(folder?.as)) add(`Two folder permissions use the same name "${folder.as}".`);
    mounts.push(folder?.as);
  }
  for (const host of manifest.permissions?.http ?? []) {
    const rest = String(host).trim().toLowerCase().replace(/^https?:\/\//, "").replace(/^\*\./, "");
    if (!/^[a-z0-9]([a-z0-9-]*[a-z0-9])?(\.[a-z0-9]([a-z0-9-]*[a-z0-9])?)*$/.test(rest)) {
      add(`The web permission "${host}" is not valid.`);
    }
  }
  return problems;
}
