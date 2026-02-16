# OSS Porting Practice: uutils/coreutils (Rust rewrite of GNU coreutils)

Goal: First-time open source contributor learning the GitHub workflow by fixing small, beginner-friendly issues.

Repo: https://github.com/uutils/coreutils
My Fork: https://github.com/jorgitin02/coreutils

**Newbie Notes:** This is my FIRST GitHub contribution. I'm learning the full workflow: fork → clone → branch → fix → commit → push → PR.

---

## FIRST TIME CONTRIBUTOR GUIDE (Step-by-Step)

### Step 1: Fork (DONE ✓)
- Forked to: https://github.com/jorgitin02/coreutils

### Step 2: Clone YOUR fork locally
```bash
cd ~/Documents/projects
git clone https://github.com/jorgitin02/coreutils.git
cd coreutils
```

### Step 3: Add upstream (the original repo)
```bash
git remote add upstream https://github.com/uutils/coreutils.git
git remote -v   # Should show origin (your fork) and upstream (original)
```

### Step 4: Create a feature branch
```bash
git checkout -b fix-issue-XXXX   # Replace XXXX with issue number
```

### Step 5: Make your fix
- Edit the code
- Add tests
- Run `cargo test` to verify

### Step 6: Commit your changes
```bash
git add -A
git commit -m "util: brief description of fix

Fixes #XXXX"
```

### Step 7: Push to your fork

**First time only:** Set up GitHub authentication
- Option A: Use GitHub CLI: `gh auth login`
- Option B: Use HTTPS with Personal Access Token (see below)
- Option C: Set up SSH keys

```bash
git push -u origin fix-issue-XXXX
```

If prompted for username/password:
- **Username:** your GitHub username (jorgitin02)
- **Password:** your Personal Access Token (NOT your GitHub password!)

### Step 8: Open Pull Request (PR)
1. Go to https://github.com/jorgitin02/coreutils
2. GitHub will show a "Compare & pull request" button
3. Click it, fill in the description, submit!

---

## Understanding GitHub Vocabulary

| Term | What it means |
|------|---------------|
| **Fork** | Your personal copy of someone else's repo (on GitHub) |
| **Clone** | Download your fork to your local machine |
| **Branch** | A separate line of development for your fix |
| **Commit** | Save your changes with a message |
| **Push** | Upload your commits to GitHub |
| **PR (Pull Request)** | Ask the maintainers to "pull" your changes into their repo |
| **Upstream** | The original repo (uutils/coreutils) |
| **Origin** | Your fork (jorgitin02/coreutils) |

---

## Common First-Timer Mistakes to Avoid

1. **DON'T commit to `main` branch** - Always create a feature branch
2. **DON'T edit files directly on GitHub** - Use your local clone
3. **DON'T work on multiple issues in one branch** - One branch per fix
4. **DO run tests before committing** - `cargo test`, `cargo fmt`, `cargo clippy`
5. **DO reference the issue** - Put "Fixes #1234" in your commit/PR

---

## Working on Multiple Fixes (One at a Time!)

**The Rule: ONE branch = ONE fix = ONE PR**

Never combine multiple fixes in one PR. Here's the workflow:

### Complete First Fix
```bash
# You're on branch fix-issue-1
git add -A
git commit -m "fix: description

Fixes #1234"
git push -u origin fix-issue-1
# Create PR on GitHub
```

### Start Second Fix
```bash
# Go back to main
git checkout main

# Get latest updates
git fetch upstream
git merge upstream/main

# Create NEW branch for second fix
git checkout -b fix-issue-2

# Make changes, commit, push, create new PR
git add -A
git commit -m "fix: another description

Fixes #5678"
git push -u origin fix-issue-2
```

### Why Separate Branches/PRs?
- Easier to review (smaller changes)
- One fix can be merged while another is being discussed
- If one fix has problems, it doesn't block the other
- Cleaner git history

---

## Pushing to GitHub (NOT main!)

**You push your FEATURE BRANCH, not main:**

```bash
# Correct - push your feature branch
git push -u origin fix-true-false-usage

# Wrong - never push directly to main
git push origin main  # DON'T DO THIS
```

The `-u` (or `--set-upstream`) links your local branch to the remote branch.

After pushing, go to https://github.com/jorgitin02/coreutils and click "Compare & pull request".

## First target issue (beginner-friendly)
Pick ONE open "good first issue" from the label list and fix it.

Suggested starter issue:
- **touch: Incorrect "months ago" calculation with `-d` options** (#10185)

(If you want an even simpler one, choose an issue that is "missing Usage:" or a small flag mismatch.)

---

## Workflow (Fork → Branch → PR)

### 1) Fork on GitHub (phone is fine)
- Open the repo page → tap **Fork**.
- This creates `yourname/coreutils`.

### 2) Clone your fork (computer)
```bash
git clone https://github.com/YOURNAME/coreutils.git
cd coreutils
```

### 3) Add upstream remote (so you can pull updates)
```bash
git remote add upstream https://github.com/uutils/coreutils.git
git remote -v
```

### 4) Create a working branch (always)
```bash
git checkout -b fix-touch-months-ago
```

---

## Development commands (Codex/OpenCode should use these)

### Build + test quickly
```bash
cargo test
```

### Format + lint (before PR)
```bash
cargo fmt
cargo clippy --all-targets --all-features
```

### Run a narrower test set (faster)
If you add/modify a test for a single utility, try running only that test module:
```bash
cargo test touch
```

(Exact test naming varies; search under `tests/` for the relevant utility and run the smallest subset you can.)

---

## How to implement a "GNU compatibility bug" fix (pattern)

1. **Reproduce**
   - Find minimal command that shows mismatch vs GNU coreutils.
   - Capture expected behavior (stdout, stderr, exit code).

2. **Locate code**
   - Utilities live in crates like `uu_<utilname>` inside the workspace.
   - Search for the util name (e.g., `touch`) and the option (`-d`).

3. **Fix**
   - Make the behavior match GNU exactly (output and exit code).
   - Prefer small, focused changes.

4. **Add a regression test**
   - Add or update a test so the bug can't come back.
   - If the project uses GNU test suite compatibility docs, mimic their behavior.

5. **Re-run**
   - `cargo test`
   - `cargo fmt`
   - `cargo clippy ...`

---

## Commit + push

```bash
git add -A
git commit -m "touch: fix relative month calculation for -d"
git push -u origin fix-touch-months-ago
```

---

## Open the PR (Pull Request)

1. Go to your fork on GitHub → it will show **Compare & pull request**.
2. Title: `touch: fix relative month calculation for -d`
3. Body (copy/paste template):

```text
Fixes #10185

What changed
- Correct relative "months ago" calculation when parsing -d options to match GNU coreutils.

How I tested
- cargo test
- (any targeted test commands you ran)

Notes
- Added regression test covering the reported case.
```

A PR is *your proposed set of changes* (often multiple commits) for maintainers to review and merge.

---

## Keeping your fork up to date (important)
Before starting a new fix:

```bash
git fetch upstream
git checkout main
git merge upstream/main
git push origin main
```

---

## "Codex/OpenCode" prompt you can reuse
Paste this into your coding assistant:

> We are contributing to uutils/coreutils (Rust rewrite of GNU coreutils).
> Task: fix issue #10185 ("touch: Incorrect months ago calculation with -d options").
> Steps: reproduce the bug, identify expected GNU behavior, locate touch's -d parsing, implement minimal fix, add regression test, run `cargo test`, `cargo fmt`, and `cargo clippy`, then prepare a PR with a clear description and "Fixes #10185".

---

## Next issues after your first PR
- Browse `good first issue` label list and pick the smallest behavior mismatch.
- Prefer: "missing flag", "usage message", "exit code mismatch", "UTF-8/bytes vs chars mismatch".

---

## GitHub Authentication (Personal Access Token)

Since you're on phone/remote, the easiest method is a Personal Access Token:

### 1. Create Token (on your phone)
1. Go to https://github.com/settings/tokens
2. Click "Generate new token (classic)"
3. Give it a name like "coreutils-dev"
4. Select scopes: **repo** (full control of private repositories)
5. Click "Generate token"
6. **COPY THE TOKEN** (you can't see it again!)

### 2. Use Token to Push
When you run `git push` and it asks for password:
- **Username:** `jorgitin02`
- **Password:** paste your token (not your GitHub password)

### 3. Cache Credentials (so you don't type every time)
```bash
git config --global credential.helper cache
# Token will be cached for 15 minutes
```
