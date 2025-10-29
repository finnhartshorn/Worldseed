# Finalise Command

You are finalizing the current work session. Follow these steps carefully:

## 1. Review Recent Changes

First, examine what has changed in this session:
- Review git status and recent commits
- Identify new features, significant changes to existing features, or important learnings about Bevy, dependencies, or development patterns

## 2. Update CLAUDE.md Documentation

Review CLAUDE.md and determine if updates are needed for:

**Update CLAUDE.md if you find:**
- New entity types, components, or systems that were added
- New modules or significant architectural changes
- New behavior patterns or AI systems
- Changes to the build process, asset structure, or development workflow
- Important learnings about Bevy 0.17 APIs, tilemaps, or sprite rendering
- New UI patterns or interaction systems
- Changes to world generation, chunk loading, or serialization
- New tile modification patterns or entity-world interactions
- Performance optimizations or important technical discoveries
- Changes to system ordering or resource management

**Do NOT update CLAUDE.md for:**
- Minor bug fixes that don't change documented behavior
- Trivial code refactoring without functional changes
- Temporary experimental code
- Changes already fully documented in CLAUDE.md

If updates are needed:
- Read the current CLAUDE.md file
- Add new information in the appropriate sections
- Maintain the existing structure and formatting style
- Be concise but thorough - include code examples where helpful
- Ensure technical accuracy (sprite dimensions, grid layouts, system ordering, etc.)
- Update system ordering section if new systems were added
- Add new development patterns if novel approaches were used

## 3. Determine Git Workflow

Check the repository setup to determine the appropriate git workflow:

```bash
# Check current branch
git branch --show-current

# Check if there are feature branches (other than main/master)
git branch -a
```

**If the repository uses feature branches:**
- Typically indicated by branches like `feature/*`, `feat/*`, `dev`, or multiple active branches
- Or if you're currently on a feature branch

**If the repository uses direct main commits:**
- Typically only has `main` or `master` branch
- Most commits go directly to main

## 4. Execute Appropriate Git Workflow

### Option A: Feature Branch Workflow (if repository uses feature branches)

1. Create a descriptive feature branch:
   ```bash
   git checkout -b docs/update-claude-md-<feature-description>
   ```
   Use a concise description like `roaming-behavior`, `tile-modification`, `new-entities`, etc.

2. Stage and commit CLAUDE.md changes:
   ```bash
   git add CLAUDE.md
   git commit -m "docs: update CLAUDE.md with <brief description>

   <More detailed description of what was documented>

   🤖 Generated with [Claude Code](https://claude.com/claude-code)

   Co-Authored-By: Claude <noreply@anthropic.com>"
   ```

3. Push the branch:
   ```bash
   git push -u origin docs/update-claude-md-<feature-description>
   ```

4. Create a pull request using gh CLI:
   ```bash
   gh pr create --title "docs: Update CLAUDE.md with <description>" --body "## Summary
   - Document <new feature/system/pattern>
   - Update <section> with <changes>

   ## Changes
   <Bulleted list of specific documentation updates>

   🤖 Generated with [Claude Code](https://claude.com/claude-code)"
   ```

### Option B: Direct Main Commit (if repository commits directly to main)

1. Stage and commit CLAUDE.md changes:
   ```bash
   git add CLAUDE.md
   git commit -m "docs: update CLAUDE.md with <brief description>

   <More detailed description of what was documented>

   🤖 Generated with [Claude Code](https://claude.com/claude-code)

   Co-Authored-By: Claude <noreply@anthropic.com>"
   ```

2. Push to main:
   ```bash
   git push origin main
   ```

## 5. Summary

Provide a brief summary to the user:
- Whether CLAUDE.md was updated (and what sections)
- Which git workflow was used (feature branch PR or direct main commit)
- The PR URL (if created) or confirmation of push to main
- Any important notes about the documentation changes

## Important Notes

- Only update CLAUDE.md if there are genuinely significant changes worth documenting
- If no documentation updates are needed, simply inform the user and skip the git workflow
- Maintain consistency with existing documentation style
- Ensure all technical specifications are accurate (frame counts, dimensions, system ordering)
- Don't remove existing documentation unless it's obsolete
- When in doubt about git workflow, check branch patterns and ask the user
