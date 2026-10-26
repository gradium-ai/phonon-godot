# Asset Store launch checklist

What is left to put Phonon Text-to-Speech on the [Godot Asset Store](https://store.godotengine.org/). The store takes an uploaded zip for each version; it does not build from the repository. Its rules are on the [guidelines page](https://store.godotengine.org/guidelines/) and in the Godot docs, [Submitting to the Asset Store](https://docs.godotengine.org/en/stable/community/asset_store/submitting_to_asset_store.html).

## Before the first submission

1. Fill in the commercial contact for the Phonon 40m models. Search for `TODO: commercial contact` in `README.md`, `addons/phonon/README.md` and `store/listing.md`.
2. Confirm the copyright holder in `LICENSE` and `addons/phonon/LICENSE` (now "Gradium").
3. Make the repository public. The listing links to its source and to `LICENSE`, and both links must work for reviewers and users.
4. Create a store account at <https://store.godotengine.org/login>, with GitHub or a new account, and decide who owns the publisher.

## Release the version

1. Make sure `version` in `Cargo.toml` is the version to release (0.1.0 for the first one), and that `main` is green.
2. Tag and push: `git tag v0.1.0 && git push origin v0.1.0`.
3. The Build workflow builds the three libraries, makes `phonon-godot-0.1.0.zip`, installs it into a new project and runs the smoke test on Linux, Windows and macOS. Then it attaches the zip to a draft GitHub release `v0.1.0`.
4. Check the draft release. Publishing it is optional; the store does not need it.

## Submit to the store

1. Click **Upload Asset**, and fill in the publisher and the asset name and URL from `store/listing.md`. Read and accept the terms of service.
2. **Settings**: summary, description, tags, asset type, license, source link and AI disclosure, from `store/listing.md`.
3. **Media**: upload `store/thumbnail.png`, then `store/screenshot-demo.png` and `store/screenshot-inspector.png`.
4. **Versions**: upload `phonon-godot-0.1.0.zip` from the release, with the name, changelog and Godot versions from `store/listing.md`.
5. **Pricing**: free.
6. **Overview**: submit for review. The store reviews each version by hand, and a rejection says why.

## Later versions

Bump `version` in `Cargo.toml` (and `Cargo.lock` with `cargo update -p phonon-godot`), tag `v<version>`, and upload the new zip on the store's **Versions** tab with its changelog.
