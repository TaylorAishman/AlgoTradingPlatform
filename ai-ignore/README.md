# AI ignore rules

`patterns` is the one list of files that AI coding tools must never index, read
or send to a model. Edit it here; every tool's ignore file at the repository
root is a symlink to it, because most tools only look for their file there.

| Root link | Tool |
|-----------|------|
| `.cursorignore` | Cursor |
| `.aiderignore` | Aider |
| `.codeiumignore` | Windsurf / Codeium |
| `.geminiignore` | Gemini CLI |
| `.aiexclude` | Gemini Code Assist, Android Studio |
| `.continueignore` | Continue |
| `.clineignore` | Cline |
| `.rooignore` | Roo Code |
| `.aiignore` | JetBrains AI Assistant, Junie |
| `.copilotignore` | Copilot (not read by GitHub itself: also set Content exclusion in the repository's Copilot settings) |

Tools without an ignore file follow written rules instead: `CLAUDE.md`,
`AGENTS.md` and `.github/copilot-instructions.md`. Claude Code also has hard
deny rules in `.claude/settings.json`.

To add a tool, link its file to the list:

```sh
ln -s ai-ignore/patterns .newtoolignore
```

On Windows, enable symlinks in git (`git config core.symlinks true`, with
Developer Mode on) or the links check out as plain text files.

These files only ask tools not to look. The real protection is that the keys
file is private (mode 600) and never committed.
