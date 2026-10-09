# Skill provenance

Installed Replica Universal into `.agents/skills/` from
https://github.com/Itskorrah/replica-skill-universal, using its Python installer.
Applied recon, architect, design, build, backend, test, diff, and brand methods
within the already-authorized desktop scope. Website deployment and commercial
accounts were not needed. Desktop-specific architecture replaces web defaults.

Read https://www.skills.sh/vercel-labs/skills/find-skills and its upstream
https://github.com/vercel-labs/skills/blob/main/skills/find-skills/SKILL.md.
Checked the public leaderboard. `npx skills find rust` and `npx skills find tauri`
returned no results in this environment. Installed the official Anthropic
`frontend-design` skill using `npx skills add anthropics/skills --skill
frontend-design --agent codex --yes`; read before applying it.

No skill substitutes for native OS documentation or real hardware validation.

2026-10-09: Used find-skills again, checked the public leaderboard, and ran
`npx skills find 'rust tauri'`. Reviewed the source and directory audit metadata
for `nodnarbnitram/claude-code-extensions@tauri-v2` (7.6K directory installs,
16 GitHub stars at review), then installed it locally with the skills CLI.
Applied its IPC/state/capability guidance and checked native API choices against
Tauri, Apple SDK headers, and Microsoft documentation. Used the Vercel React
best-practices checklist for UI edits. Skills do not establish OS correctness.
