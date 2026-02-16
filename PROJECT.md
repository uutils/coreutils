# Project: First GitHub Contribution to uutils/coreutils

## Overview
This is your first open-source contribution. This guide assumes you've never contributed to GitHub before and walks you through every step, explaining WHY we do each thing.

---

## What You'll Learn
- How to contribute to open-source projects on GitHub
- Forking, branching, and pull requests
- Basic Rust development workflow
- Testing and linting with cargo

---

## Prerequisites Checklist
Before starting, make sure you have:

- [ ] **A GitHub account** - Sign up at https://github.com if you haven't
- [ ] **Git installed** - Check with: `git --version`
- [ ] **Rust installed** - Check with: `rustc --version` and `cargo --version`
- [ ] **A terminal** - Terminal.app, iTerm2, or VS Code terminal

### First-time Git setup (one-time)
If you've never used git before, set your identity:

```bash
git config --global user.name "Your Name"
git config --global user.email "you@example.com"
```

---

## The Contribution Workflow (Explained)

### Understanding the Architecture
```
Original Repository (uutils/coreutils)
           |
           v
    [You FORK it] → Creates YOUR copy on GitHub
           |
           v
    [You CLONE it] → Downloads to your computer
           |
           v
    [Create BRANCH] → Isolated workspace for your fix
           |
           v
    [Make CHANGES] → Edit code, add tests
           |
           v
    [Push to YOUR fork] → Upload your branch
           |
           v
    [Open PULL REQUEST] → Ask maintainers to review
```

---

## Step-by-Step Instructions

### Phase 1: Setup (One-time per project)

#### Step 1: Fork the Repository
**What is forking?** Creating your own copy of the project on GitHub.

1. Go to https://github.com/uutils/coreutils
2. Click the **Fork** button (top right)
3. Select your account
4. Wait for the fork to complete
5. You'll be redirected to `https://github.com/YOUR-USERNAME/coreutils`

**Why fork?** You need permission to push changes. The original repo is read-only for you.

#### Step 2: Clone Your Fork
**What is cloning?** Downloading your fork to your computer.

```bash
# Navigate to where you want the project
cd ~/Documents/projects

# Clone YOUR fork (replace YOUR-USERNAME)
git clone https://github.com/YOUR-USERNAME/coreutils.git

# Enter the project directory
cd coreutils
```

**What happened?** You now have a complete copy of the project on your computer.

#### Step 3: Add the Upstream Remote
**What is a remote?** A pointer to a repository location (URL).

```bash
# Add the original repository as "upstream"
git remote add upstream https://github.com/uutils/coreutils.git

# Verify you have two remotes:
git remote -v
```

**Expected output:**
```
origin    https://github.com/YOUR-USERNAME/coreutils.git (fetch)
origin    https://github.com/YOUR-USERNAME/coreutils.git (push)
upstream  https://github.com/uutils/coreutils.git (fetch)
upstream  https://github.com/uutils/coreutils.git (push)
```

**Why add upstream?** To get updates from the original project before starting new work.

---

### Phase 2: Make Your First Fix

#### Step 4: Find a Good First Issue
1. Go to https://github.com/uutils/coreutils/issues
2. Click **Labels** → Select **"good first issue"**
3. Pick one that seems manageable (we suggest #10185 - touch months ago)

#### Step 5: Create a Branch
**What is a branch?** An isolated workspace for your changes.

```bash
# Make sure you're on main first
git checkout main

# Create and switch to a new branch
git checkout -b fix-touch-months-ago
```

**Why branches?**
- Keeps your main branch clean
- Allows multiple fixes in parallel
- Makes PR review easier (only shows your changes)

#### Step 6: Understand the Project Structure

```
coreutils/
├── Cargo.toml          # Workspace configuration
├── src/                # Main binary
├── crates/
│   ├── uu_touch/       # ← touch utility code
│   │   ├── src/
│   │   │   └── touch.rs
│   │   └── Cargo.toml
│   └── uu_[util]/       # Other utilities
├── tests/              # Integration tests
└── docs/               # Documentation
```

Each utility is in its own crate named `uu_<utilname>`.

---

### Phase 3: Development

#### Step 7: Find and Fix the Code

**For issue #10185 (touch months ago):**

1. **Reproduce the bug:**
   ```bash
   # Build first
   cargo build --release

   # Test the issue
   ./target/release/touch -d "3 months ago" testfile
   ls -la testfile
   ```

2. **Find the relevant code:**
   ```bash
   # Search for "month" in the touch crate
   grep -r "month" crates/uu_touch/src/
   ```

3. **Make your fix** - Edit the file and save changes.

#### Step 8: Test Your Changes

```bash
# Run all tests (takes time)
cargo test

# Run only touch tests (faster)
cargo test touch

# Format your code
cargo fmt

# Check for common issues
cargo clippy --all-targets --all-features
```

**Fix any errors before proceeding.**

#### Step 9: Commit Your Changes

**What is a commit?** A snapshot of your changes with a message.

```bash
# Stage all modified files
git add -A

# Create a commit with a descriptive message
git commit -m "touch: fix relative month calculation for -d

Fixes #10185

The calculation was incorrectly handling month boundaries.
Now matches GNU coreutils behavior exactly."
```

**Commit message tips:**
- First line: Brief summary (50 chars or less)
- Blank line
- Body: Explain WHAT changed and WHY
- Include "Fixes #XXXX" to auto-close the issue

---

### Phase 4: Submit Your Contribution

#### Step 10: Push to Your Fork

```bash
# Push your branch to your fork on GitHub
git push -u origin fix-touch-months-ago
```

**What happened?** Your branch is now on GitHub in YOUR fork.

#### Step 11: Create a Pull Request (PR)

**What is a PR?** A request for maintainers to review and merge your changes.

1. Go to your fork: `https://github.com/YOUR-USERNAME/coreutils`
2. You'll see a banner: **"Compare & pull request"** → Click it
3. Or go to the **Pull requests** tab → **New pull request**
4. Make sure:
   - base repository: `uutils/coreutils`
   - base branch: `main`
   - head repository: `YOUR-USERNAME/coreutils`
   - compare: `fix-touch-months-ago`

5. Fill in the PR form:

**Title:** `touch: fix relative month calculation for -d`

**Body:**
```markdown
Fixes #10185

## What changed
- Corrected relative "months ago" calculation when parsing `-d` options
- Added proper handling for month boundary cases
- Now matches GNU coreutils behavior exactly

## How I tested
- `cargo test touch` - all touch tests pass
- Manual testing: `touch -d "3 months ago"` now produces correct date
- Verified against GNU coreutils behavior

## Notes
- Added regression test covering the reported case
- This is my first contribution to uutils!
```

6. Click **Create pull request**

#### Step 12: Respond to Review Feedback

**What happens next?**
- Maintainers review your code
- They may ask for changes (this is normal!)
- Make changes, commit, push again (same branch)
- The PR updates automatically

**Common feedback:**
- "Can you add a test for edge cases?"
- "Please run cargo fmt"
- "Could you explain this change?"

---

### Phase 5: Maintenance

#### Keeping Your Fork Updated

Before starting a new fix:

```bash
# Fetch updates from original repo
git fetch upstream

# Switch to main
git checkout main

# Merge upstream changes
git merge upstream/main

# Push to your fork
git push origin main
```

---

## Common Beginner Mistakes & Fixes

### Mistake 1: Committed to main instead of a branch
**Fix:**
```bash
# Save your changes to a new branch
git checkout -b my-fix-branch

# Reset main to match origin
git checkout main
git reset --hard origin/main
```

### Mistake 2: Forgot to sync before starting work
**Fix:**
```bash
git fetch upstream
git rebase upstream/main
# Resolve any conflicts if they appear
```

### Mistake 3: Committed with wrong email
**Fix:**
```bash
# Update your git config
git config user.email "correct@example.com"

# Amend the last commit
git commit --amend --no-edit
```

### Mistake 4: Tests fail but you didn't change anything
**Possible causes:**
- You need to run `cargo build` first
- Rust version mismatch (check with `rustc --version`)
- Forgot to save your file before testing

---

## Git Commands Quick Reference

| Command | What it does |
|---------|--------------|
| `git status` | See what's modified/staged |
| `git log --oneline` | See commit history |
| `git diff` | See unstaged changes |
| `git add <file>` | Stage a file for commit |
| `git add -A` | Stage all changes |
| `git commit -m "msg"` | Create a commit |
| `git push` | Upload commits to GitHub |
| `git pull` | Download updates from GitHub |
| `git branch` | List branches |
| `git checkout <branch>` | Switch branches |

---

## When You're Stuck

1. **Check the issue page** - Others may have posted solutions
2. **Read existing PRs** - See how others solved similar issues
3. **Ask in the issue** - "I'm working on this, could someone clarify..."
4. **Search the codebase** - Look for similar patterns in other utilities
5. **Take a break** - Fresh eyes help solve problems

---

## Success Criteria

Your first contribution is complete when:
- [ ] PR is created with clear description
- [ ] All CI checks pass (green checkmarks)
- [ ] You respond to any review feedback
- [ ] PR is merged by a maintainer

**Time to celebrate!** You've made your first open-source contribution.

---

## Next Steps After Your First PR

1. **Update your fork's main branch**
2. **Find another "good first issue"**
3. **Consider harder issues** as you gain confidence
4. **Help others** in issue discussions

---

## Resources

- **uutils Contributing Guide:** https://github.com/uutils/coreutils/blob/main/CONTRIBUTING.md
- **Rust Book:** https://doc.rust-lang.org/book/
- **GitHub Flow:** https://docs.github.com/en/get-started/quickstart/github-flow
- **Git Documentation:** https://git-scm.com/doc

---

## Your Progress

### First Fix: Issue #10279 - true/false Usage output ✅ PUSHED

**Issue:** https://github.com/uutils/coreutils/issues/10279
**Branch:** `fix-true-false-usage`
**Status:** Code pushed, create PR now!

**What was fixed:**
- Modified `src/uu/true/src/true.rs` to show "Usage:" first in help output
- Modified `src/uu/false/src/false.rs` to show "Usage:" first in help output
- Changed help template from `{about}\n\nUsage:...` to `Usage:...\n\n{about}`

**Before:**
```
Returns true, a successful exit status.
...
Usage: true
```

**After:**
```
Usage: true

Returns true, a successful exit status.
...
```

**Tests:**
- [x] cargo build passes
- [x] cargo fmt passes
- [x] cargo clippy passes
- [ ] Integration tests pass

**Next Steps:**
1. ✅ Push branch to GitHub
2. 🔄 **Create PR now:** https://github.com/jorgitin02/coreutils/pull/new/fix-true-false-usage
3. ⏳ Wait for maintainer review

**PR Title:** `true/false: fix help output to start with Usage (matching GNU)`

**PR Body:**
```
Fixes #10279

This PR fixes the --help output for `true` and `false` commands to start with "Usage:" line, matching GNU coreutils behavior.

Changes:
- Modified help template in both true.rs and false.rs
- Changed from "{about}...Usage:" to "Usage:...{about}" format
- Both commands now show "Usage: true/false" as the first line

Testing:
- cargo build passes
- cargo fmt passes
- cargo clippy passes
```
