# Security Policy

## Supported Versions

Only the latest release and the current `main` branch are actively supported with security updates.

| Version | Supported          |
| ------- | ------------------ |
| >= 0.1.0 | :white_check_mark: |
| < 0.1.0  | :x:                |

---

## Reporting a Vulnerability

The multiplay team takes the security of our application, network protocols, and our users' audio systems seriously.

If you believe you have discovered a vulnerability (such as remote code execution, unauthenticated remote command injection, buffer overflows, memory safety issues, or sensitive credential leakage), please report it responsibly:

1. **Do not disclose publicly:** Please do NOT open a public GitHub issue or discuss the vulnerability in public channels.
2. **Contact:** Email security reports to `security@multiplay-audio.internal` (or submit a private security advisory on GitHub if available).
3. **Information to Include:**
   - A detailed description of the vulnerability.
   - Steps to reproduce or proof-of-concept exploit code.
   - The impact on the user or system (e.g., local privilege escalation, network control, denial-of-service).
   - Any mitigation ideas or proposed fixes.

---

## Response Process

- We will acknowledge receipt of your vulnerability report within 48 hours.
- We will work with you to understand and validate the report.
- Once a fix is verified, a patch release and advisory will be coordinated.

---

## Known Security Architecture Notes

- **Remote Daemon:** By default, remote daemon listening is restricted or bound to loopback `127.0.0.1` pending cryptographic token validation (`PLAN-035`).
- **Memory Safety:** `#![forbid(unsafe_code)]` is enforced across all domain and engine modules.
