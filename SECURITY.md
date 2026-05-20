# Security Policy

stark-v is a work in progress and has not yet been audited. It is not
recommended for production use. That said, security reports — including for the
pre-production code — are taken seriously and handled privately.

## Reporting a vulnerability

Please **do not** open a public GitHub issue for security problems.

Report vulnerabilities by email to
**[clement0walter@gmail.com](mailto:clement0walter@gmail.com)**, or via GitHub's
[private vulnerability reporting](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing-information-about-vulnerabilities/privately-reporting-a-security-vulnerability)
on the repository.

A useful report typically contains:

- A description of the issue and the component it affects (prover, verifier,
  runner, opcode AIR, guest runtime, …).
- The version, commit hash or branch the report is based on.
- A minimal reproduction: input, code snippet, or PoC.
- The impact you believe the issue has (soundness, completeness, DoS,
  information leak).
- Any suggested mitigation, if you have one.

We will acknowledge receipt within 5 business days and aim to provide an initial
assessment within 10 business days.

## Scope

In scope:

- Soundness or completeness issues in the prover or verifier
- Constraint gaps allowing a forged trace to be accepted
- Memory safety issues (`unsafe` blocks, FFI, etc.)
- Issues in the guest ABI that allow a malicious guest to violate isolation
- Supply-chain concerns in the build or release process

Out of scope:

- Performance characteristics that are not exploitable
- Issues in third-party dependencies that are tracked upstream — please report
  those to the upstream project (a heads-up here is still welcome if stark-v is
  materially affected)
- Findings against the contents of `external/` (vendored submodules) — please
  report those to the upstream project

## Disclosure

We follow coordinated disclosure: once a fix is available, we will publish a
security advisory crediting the reporter (unless anonymity is requested) and
release a patched version. Please give us a reasonable window — typically 90
days — before public disclosure.

## Safe harbour

Good-faith security research that follows this policy will not be pursued. "Good
faith" means: no exfiltration of third-party data, no denial of service against
shared infrastructure, no public disclosure before a fix is available.
