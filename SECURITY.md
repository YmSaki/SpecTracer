# Security Policy

SpecTracer is an OSS verification tool. Send vulnerability details and sensitive
project data through private reporting, because a public issue is visible to
everyone, including attackers, before a fix exists. When private reporting is
unavailable, follow the fallback below.

## Supported versions

| Version | Supported |
|---|---|
| Latest GitHub release | Yes |
| `main` | Best effort during active development |
| Older releases | No, unless explicitly stated in the release notes |

## Reporting a vulnerability

Use [GitHub private vulnerability reporting](https://github.com/YmSaki/SpecTracer/security/advisories/new)
when it is enabled for the repository. Include the affected version or commit,
the smallest reproducible example, impact, and any suggested mitigation. Please
remove credentials, proprietary source, and personal data from the report.

If private reporting is unavailable, open a public issue containing only the
word “security” and a request for a private contact channel; share exploit details
through the private channel once it is agreed.

Maintainers will acknowledge a report when practical, investigate it against the
canonical source and release artifacts, and document a fix or mitigation in the
release notes. Release archives include SHA-256 checksum files; verify those
checksums before distributing a downloaded binary.
