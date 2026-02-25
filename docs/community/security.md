# Security Policy

## Reporting Vulnerabilities

If you discover a security vulnerability in GlowBack, **do not** open a public
issue. Instead:

1. Use GitHub's [private vulnerability reporting](https://github.com/LatencyTDH/GlowBack/security)
   feature, or
2. Contact the maintainers directly.

We will acknowledge receipt within 48 hours and target a fix within 7 days for
critical issues.

## Scope

This policy covers:

- The Rust engine crates (`crates/`)
- Python bindings (`crates/gb-python/`)
- FastAPI gateway
- Streamlit UI

Third-party dependency vulnerabilities are tracked via Dependabot.

## Supported Versions

Only the `main` branch receives security updates.
