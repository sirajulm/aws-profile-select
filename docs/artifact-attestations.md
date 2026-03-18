# Artifact Attestations [TODO]

GitHub artifact attestations let you cryptographically prove where and how your software was built, establishing build provenance for binaries and container images. This is primarily useful for projects with compliance requirements (e.g. SLSA) or widely-depended-on packages where users need to verify that a downloaded artifact was genuinely produced by your CI pipeline. As of now, `aws-profile-select` does not require attestations since it is a CLI tool distributed directly via GitHub Releases through `cargo-dist`, and the blast radius of a compromised build is small. If the project grows to publish to registries like crates.io or Homebrew, or gains a significant user base that needs provenance guarantees, adding attestation would be straightforward — it requires the `attestations: write` permission and an `actions/attest-build-provenance` step in the release workflow. Revisit this decision if supply-chain security requirements change.

For full setup instructions, see the official guide:
<https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations>
