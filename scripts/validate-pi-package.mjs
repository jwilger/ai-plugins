#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const root = path.resolve(import.meta.dirname, "..");
const inventory = JSON.parse(
  fs.readFileSync(path.join(root, ".agents/plugins/pi-support.json"), "utf8"),
);
if (inventory.schemaVersion !== 1 || !Array.isArray(inventory.packages)) {
  throw new Error("unsupported Pi support inventory");
}

const names = new Set();
for (const entry of inventory.packages) {
  if (names.has(entry.name))
    throw new Error(`duplicate Pi package: ${entry.name}`);
  names.add(entry.name);
  const packageRoot = path.resolve(root, entry.path);
  const manifest = JSON.parse(
    fs.readFileSync(path.join(packageRoot, "package.json"), "utf8"),
  );
  const extension = manifest.pi?.extensions ?? [];
  const skills = manifest.pi?.skills ?? [];
  if (extension.length !== 1 || extension[0] !== entry.extension) {
    throw new Error(
      `${entry.name}: extension manifest differs from Pi support inventory`,
    );
  }
  const declaredSkillNames = skills
    .map((skillPath) => path.basename(skillPath))
    .sort();
  const expectedSkillNames = [...entry.skills].sort();
  if (
    JSON.stringify(declaredSkillNames) !== JSON.stringify(expectedSkillNames)
  ) {
    throw new Error(
      `${entry.name}: public skill manifest differs from Pi support inventory`,
    );
  }
  if (new Set(declaredSkillNames).size !== declaredSkillNames.length) {
    throw new Error(`${entry.name}: duplicate public skill name`);
  }
  for (const resource of [
    ...extension,
    ...skills,
    ...entry.componentEntrypoints,
  ]) {
    const absolute = path.resolve(packageRoot, resource);
    if (!fs.existsSync(absolute))
      throw new Error(`${entry.name}: missing resource ${resource}`);
  }
  for (const skillName of entry.skills) {
    const skillFile = path.join(packageRoot, "skills", skillName, "SKILL.md");
    const source = fs.readFileSync(skillFile, "utf8");
    const frontmatterName = source.match(
      /^---\n[\s\S]*?^name:\s*([^\s]+)\s*$/m,
    )?.[1];
    if (frontmatterName !== skillName) {
      throw new Error(
        `${entry.name}: skill collision or invalid name at ${skillName}`,
      );
    }
  }
}

const developmentSystem = inventory.packages.find(
  (entry) => entry.name === "development-system",
);
if (!developmentSystem) {
  throw new Error("development-system: Pi support inventory entry missing");
}
const facade = JSON.parse(
  fs.readFileSync(path.join(root, "package.json"), "utf8"),
);
const facadePath = (relative) =>
  `./${path.posix.join(
    developmentSystem.path.replace(/^\.\//, ""),
    relative.replace(/^\.\//, ""),
  )}`;
const expectedFacade = {
  extensions: [facadePath(developmentSystem.extension)],
  skills: developmentSystem.skills.map((skill) =>
    facadePath(`skills/${skill}`),
  ),
};
const peerMetadata = Object.values(facade.peerDependenciesMeta ?? {});
if (
  facade.private !== true ||
  Object.hasOwn(facade, "dependencies") ||
  Object.hasOwn(facade, "devDependencies") ||
  peerMetadata.length !== Object.keys(facade.peerDependencies ?? {}).length ||
  peerMetadata.some((metadata) => metadata?.optional !== true)
) {
  throw new Error(
    "development-system: root Pi package facade must be private and lightweight",
  );
}
if (JSON.stringify(facade.pi) !== JSON.stringify(expectedFacade)) {
  throw new Error(
    "development-system: root Pi package facade differs from support inventory",
  );
}

console.log(
  `development_system.pi_inventory packages=${inventory.packages.length} valid=true`,
);
