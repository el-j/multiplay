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
2. **Contact:** Submit a private report via GitHub Security Advisories: <https://github.com/el-j/multiplay/security/advisories/new>.
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

- **Remote Daemon:** off by default; loopback-only unless LAN access is enabled. Upgrades require a single-use pairing token or a 128-bit session secret (stored hashed), a matching `Origin`, and an IP-literal/localhost `Host`. Failed attempts are throttled per address, and peers (8), frame size (64 KiB) and handshake time (10 s) are capped. Remotes can only reference tracks the host already has. Builds before PLAN-035 (commit `7a9a891` and earlier) started an unauthenticated server on `0.0.0.0:9002`.
- **Memory Safety:** `#![forbid(unsafe_code)]` is enforced in the domain and core engine crates.
