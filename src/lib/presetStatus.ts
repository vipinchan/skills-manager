import type { ManagedSkill, Preset } from "./tauri";

export type PresetStatus = "active" | "partial" | "inactive" | "empty";

export interface PresetStatusResult {
  status: PresetStatus;
  installed: number;
  total: number;
}

function centralDirName(skill: ManagedSkill) {
  return skill.central_path.split(/[\\/]/).filter(Boolean).pop() ?? skill.name;
}

export function canonicalPresetSkills(preset: Preset, skills: ManagedSkill[]) {
  const presetSkills = skills.filter((s) => s.preset_ids.includes(preset.id));
  const byName = new Map<string, ManagedSkill[]>();
  for (const skill of presetSkills) {
    const group = byName.get(skill.name) ?? [];
    group.push(skill);
    byName.set(skill.name, group);
  }

  return Array.from(byName.values()).map((group) => {
    if (group.length === 1) return group[0];
    return group.find((skill) => centralDirName(skill) === skill.name) ?? group[0];
  });
}

export function computePresetStatus(
  preset: Preset,
  skills: ManagedSkill[],
  agentKeys: string[],
  existsInWorkspace: (skill: ManagedSkill, agentKey: string) => boolean
): PresetStatusResult {
  const presetSkills = canonicalPresetSkills(preset, skills);
  if (presetSkills.length === 0 || agentKeys.length === 0) {
    return { status: "empty", installed: 0, total: 0 };
  }
  const total = presetSkills.length * agentKeys.length;
  let installed = 0;
  for (const skill of presetSkills) {
    for (const agentKey of agentKeys) {
      if (existsInWorkspace(skill, agentKey)) installed++;
    }
  }
  if (installed === total) return { status: "active", installed, total };
  if (installed === 0) return { status: "inactive", installed, total };
  return { status: "partial", installed, total };
}
