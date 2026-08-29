# Security Policy

## Supported Versions

We provide security updates only for the latest released version of `uutils/coreutils`.
Older versions may not receive patches.
If you are using a version packaged by your Linux distribution, please check with your distribution maintainers for their update policy.

---

## Threat Model and Scope

`uutils/coreutils` is a set of **local** command-line utilities. There is no
network-facing service, so remote vulnerabilities do not apply. When assessing
whether a report is a security issue, we focus on whether it crosses a trust or
privilege boundary on the local system.

**In scope** (treated as security issues):

- **Privilege-boundary violations** - bypassing a documented safety guard
  (e.g. `--preserve-root`), acting on files outside the intended scope, or
  privilege escalation in setuid/sudo contexts.
- **Filesystem race conditions** - TOCTOU bugs, symlink following, and unsafe
  directory traversal in utilities that recurse or operate on paths
  (`cp`, `mv`, `rm`, `chown`, `chmod`, `install`, …). *TOCTOU*
  (Time-Of-Check to Time-Of-Use) is when a utility checks a file's state
  (existence, type, permissions) and then acts on it in a separate step, leaving
  a window for an attacker to swap the path in between - e.g. replacing a regular
  file with a symlink so the action lands on a different target. *Safe
  traversal* is the defense: when walking a directory tree, descend with
  symlink-aware primitives (`openat`/`O_NOFOLLOW`, file descriptors instead of
  re-resolved paths) so a symlink swapped in mid-walk cannot redirect the
  operation outside the intended tree.
- **Unintended destructive actions** - operations that delete, overwrite, or
  signal something the user did not ask for, including security-relevant
  divergences from GNU behavior that scripts rely on (a parsing bug that "fails
  open" instead of erroring).
- Memory-safety issues, integer overflow, or unbounded allocation reachable
  from untrusted input.

**Out of scope** (report as a normal bug, not a security issue):

- Crashes, panics, or incorrect output with no privilege or safety impact.
- Cosmetic differences from GNU (messages, exit-code-only mismatches) that do
  not change which files or processes are affected.
- Issues requiring an already-privileged or already-malicious local actor who
  could achieve the same effect directly.

### Local denial-of-service

Because these are local tools, resource exhaustion (hang, infinite loop,
unbounded memory/CPU, crash on crafted input) is only a security issue when the
triggering input **crosses a trust boundary**. A user who runs a utility on their
own data and exhausts their own resources is harming only their own invocation -
that is a normal bug. It is in scope when crafted, untrusted input reaches the
utility through a boundary the victim does not control, such as a privileged
script, a cron job, or an automated pipeline that processes attacker-influenced
filenames or file contents and is thereby wedged, blocked, or terminated.

Severity reflects impact, not just whether a bug exists: a guard bypass that can
hit the whole filesystem is critical, while a local availability issue bounded by
the caller's existing permissions is low.

---

## Reporting a Vulnerability

**Do not open public GitHub issues for security vulnerabilities.**
This prevents accidental disclosure before a fix is available.

Please use one of the following methods:

- **GitHub (preferred):** open a private report at
  <https://github.com/uutils/coreutils/security/advisories/new>
  ("Report a vulnerability").
- **Email:** [sylvestre@debian.org](mailto:Sylvestre@debian.org)
- **Encryption (optional):** You may encrypt your report using our PGP key:
Fingerprint: B60D B599 4D39 BEC4 D1A9 5CCF 7E65 28DA 752F 1BE1
---

### What to Include in Your Report

To help us investigate and resolve the issue quickly, please include as much detail as possible:

- **Type of issue:** e.g. privilege escalation, information disclosure.
- **Location in the source:** file path, commit hash, branch, or tag.
- **Steps to reproduce:** exact commands, test cases, or scripts.
- **Special configuration:** any flags, environment variables, or system setup required.
- **Affected systems:** OS/distribution and version(s) where the issue occurs.
- **Impact:** your assessment of the potential severity (DoS, RCE, data leak, etc.).

---

## Disclosure Policy

We follow a **Coordinated Vulnerability Disclosure (CVD)** process:

1. We will acknowledge receipt of your report within **10 days**.
2. We will investigate, reproduce, and assess the issue.
3. We will provide a timeline for developing and releasing a fix.
4. Once a fix is available, we will publish a GitHub Security Advisory.
5. You will be credited in the advisory unless you request anonymity.
