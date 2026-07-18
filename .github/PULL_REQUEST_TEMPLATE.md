<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- Meltemi contributions are spec-driven. See CONTRIBUTING.md. -->

## Linked change

<!-- A feature PR MUST link its approved change proposal. -->
Change: <!-- e.g. add-thing, or "fast track: typo fix" -->

## Summary

<!-- What this PR does, in a sentence or two. -->

## Type

- [ ] Feature/change (linked change proposal above, with its artifacts)
- [ ] Fast track (trivial: typo/docs/formatting/one-line fix)

## Quality checklist

- [ ] The linked change's scenarios are covered by tests or a documented
      verification.
- [ ] `cargo clippy -- -D warnings` is clean.
- [ ] `cargo fmt --check` is clean.
- [ ] Tests pass on the three platforms (Windows, macOS, Linux).
- [ ] Every touched source file has its SPDX header.
- [ ] Dependencies (if any new) are minimal, pinned, and justified in the design.
- [ ] Commits are atomic, reference the change and task, and carry **no
      co-authorship trailer**.

<!--
A feature PR without a linked change will be asked to open a proposal first.
Co-authored-by trailers are not accepted.
-->
