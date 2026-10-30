#!/usr/bin/env node
/**
 * Refuses to build a public release from the wrong state.
 * Run by `npm run dist` and by the release workflow.
 */
import { execSync } from "node:child_process";

const allowDirty = process.argv.includes("--allow-dirty");
const allowUntagged = process.argv.includes("--allow-untagged");
const ci = process.env.GITHUB_ACTIONS === "true";

const git = (cmd) => execSync(`git ${cmd}`, { encoding: "utf8" }).trim();

const problems = [];

if (process.env.PI_CHANNEL === "dev") {
  problems.push("PI_CHANNEL=dev — a release must be built with PI_CHANNEL=release");
}

const dirty = git("status --porcelain");
if (dirty && !allowDirty) {
  problems.push(`working tree is not clean:\n${dirty.split("\n").slice(0, 10).map((l) => "    " + l).join("\n")}`);
}

const tag = (() => {
  try {
    return git("describe --tags --exact-match HEAD");
  } catch {
    return "";
  }
})();
if (!tag && !allowUntagged) {
  problems.push("HEAD has no tag — releases come from a tagged commit (git tag vX.Y.Z)");
}

const version = JSON.parse(execSync("node -p 'JSON.stringify(require(\"./package.json\"))'", { encoding: "utf8" })).version;
if (tag && tag !== `v${version}`) {
  problems.push(`tag ${tag} does not match package.json version ${version}`);
}

if (problems.length) {
  console.error("\n✗ release guard: refusing to build\n");
  for (const p of problems) console.error("  • " + p);
  console.error("\n  use `npm run dist:unsafe` to bypass locally (never in CI)\n");
  process.exit(1);
}

console.log(`✓ release guard: ${tag || "untagged"} @ ${git("rev-parse --short HEAD")}${ci ? " (CI)" : ""}`);
