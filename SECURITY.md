# Security Policy

## Supported Versions

We provide security updates only for the latest released version of `uutils/coreutils`.
Older versions may not receive patches.
If you are using a version packaged by your Linux distribution, please check with your distribution maintainers for their update policy.

---

## Reporting a Vulnerability

**Do not open public GitHub issues for security vulnerabilities.**
This prevents accidental disclosure before a fix is available.

Instead, please use the following method:

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
