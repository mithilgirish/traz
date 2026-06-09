# Security Policy

Security is a priority for the `traz` project. As a local-first engineering memory layer and MCP server, we are committed to ensuring the safety and privacy of your data and workflows. This document outlines our security policy, including supported versions, how to report vulnerabilities, and our incident response process.

## Supported Versions

The following table lists the versions of `traz` that are currently being supported with security updates. We highly recommend keeping your installation up to date.

| Version | Supported          | Notes |
| ------- | ------------------ | ----- |
| 0.1.x   | :white_check_mark: | Active development and support. |
| < 0.1   | :x:                | Unsupported. Please upgrade to 0.1.x. |

*Note: Since `traz` is currently in its early stages (0.x.x), we only provide security patches for the latest minor release.*

## Reporting a Vulnerability

**Please do not open a public GitHub issue or pull request for a security vulnerability.**

Instead, report any suspected security vulnerabilities privately. You can do this by using the **GitHub Security Advisories** feature:
1. Navigate to the [Security Advisories tab](https://github.com/mithilgirish/traz/security/advisories) in the repository.
2. Click on **"Report a vulnerability"**.
3. Fill out the form with as much detail as possible.

### What to Include in Your Report
To help us understand and resolve the issue quickly, please include:
- **Description**: A clear summary of the vulnerability and its impact.
- **Steps to Reproduce**: Detailed steps on how to trigger the vulnerability.
- **Environment Details**: OS, `traz` version, and any relevant configuration or integrations.
- **Potential Impact**: How this vulnerability could be exploited and what the consequences are.
- **Proof of Concept (PoC)**: (Optional but highly recommended) Scripts, screenshots, or logs demonstrating the issue.

## Vulnerability Handling Process

Once a vulnerability is reported, our team will follow this process:

1. **Triage**: We aim to acknowledge your report within **48 hours**. We will verify the vulnerability and assess its severity.
2. **Remediation**: If the vulnerability is confirmed, we will begin working on a fix. We will keep you updated on our progress and the estimated timeline for a patch.
3. **Patch Release**: A security patch will be developed, tested, and released. We may backport the fix to supported older versions if applicable.
4. **Disclosure**: After the patch has been released and users have been given reasonable time to upgrade, we will publish a public security advisory and credit you for the discovery (unless you prefer to remain anonymous).

## Out of Scope

Since `traz` operates as a local-first application and CLI tool, the following scenarios are generally considered out of scope for security vulnerabilities:
- **Physical Access**: Attacks requiring physical access to the user's unlocked device.
- **Compromised Local Account**: Attacks relying on the user's operating system account already being compromised by malware or a malicious actor.
- **Local File System Permissions**: Issues stemming from the user intentionally misconfiguring their local file system permissions.
- **Third-Party API Keys**: Leaks of user-provided API keys (e.g., Anthropic, OpenAI) if they are explicitly shared or committed by the user outside of `traz`'s configuration mechanisms.

We appreciate your efforts to responsibly disclose vulnerabilities and help keep the `traz` community secure!
