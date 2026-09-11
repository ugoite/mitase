---
title: "Release candidate identity"
description: "The reproducible identity contract for Mitase release candidates."
sidebar_position: 4
---

# Release candidate identity

Mitase release artifacts are promoted from a candidate, not rebuilt from an
incidental branch or tag. A candidate records the exact source revision and
the exact bytes selected for publication.

The repository's public version authority is the `version` in the Cargo
workspace package section of [Cargo.toml](../../Cargo.toml). The Foundation
Release is `v0.1.0`; the current patch train is prepared as `v0.1.2`. The
candidate workflow reads the version from the exact source revision it checks
out.

## Manifest

The candidate manifest uses the `mitase/release-candidate/v1` schema:

```json
{
  "artifacts": [
    {
      "name": "mitase-v0.1.0-x86_64-unknown-linux-gnu.tar.gz",
      "sha256": "sha256:<64 lowercase hexadecimal characters>"
    }
  ],
  "candidate_id": "sha256:<64 lowercase hexadecimal characters>",
  "schema": "mitase/release-candidate/v1",
  "source_sha": "<40 lowercase hexadecimal Git commit characters>",
  "version": "v0.1.0"
}
```

`artifacts` is sorted by `name`. The canonical manifest encoding is UTF-8
JSON with sorted object keys, compact separators, and one trailing newline.
The `candidate_id` is the SHA-256 digest of that canonical encoding with the
`candidate_id` field omitted. It therefore identifies the version, exact
source commit, and complete artifact digest set without a circular hash.

## Boundary

`scripts/ci/release_candidate.py` is repository release tooling. It validates
the source commit and artifact inputs and creates or checks the manifest. It
does not build software, execute tests, publish artifacts, promote releases,
or add release lifecycle responsibilities to the Mitase executable.

The later release workflow must build from the recorded `source_sha`, generate
the manifest after packaging, verify it before publication, and promote those
same bytes. Version tags and mutable aliases are human-facing selectors; they
are not substitutes for the candidate or its digests.

## Delivery workflow

The `release-candidate` workflow is manually dispatched with an exact source
SHA. It runs the release-grade repository gate, builds and packages each
target, and uploads the candidate bundle only after the manifest and artifact
digests have been verified.

The separate `release-publish` workflow is manually dispatched with the
candidate workflow run ID and the recorded `candidate_id`. It downloads that
bundle, verifies the manifest checksum, candidate identity, source revision,
and every artifact byte, then publishes those same files to the GitHub release
and package registry. It does not rebuild from `main` or from a tag.

The release-line behavior and the v0.2 cutover acceptance are fixed in the
[release train acceptance](./release-acceptance.md) contract. The candidate
workflow runs that repository-only gate after the standard `mise run ci`
quality gate.

## Standalone CLI package

Each published release contains one versioned Unix archive for each supported
target:

```text
mitase-v<version>-x86_64-unknown-linux-gnu.tar.gz
mitase-v<version>-aarch64-unknown-linux-gnu.tar.gz
mitase-v<version>-x86_64-apple-darwin.tar.gz
mitase-v<version>-aarch64-apple-darwin.tar.gz
```

An archive contains exactly one file, the executable `mitase`. The candidate
workflow smoke-tests the exact packaged archive on a native runner by checking
`mitase --version` and running `mitase check fixtures/acceptance/ugoite-foundation-policy-v1`. The
published `SHA256SUMS` and `release-manifest.json` describe and verify those
same archive bytes; downstream repositories may pin those immutable version
and digest values without importing Mitase crates or source.
The installer supports these four Unix targets only and reports unsupported
operating systems before attempting a download.
