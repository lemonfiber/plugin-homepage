# AGENTS.md — plugin-homepage

> **Start at the roadmap and board on [lemonfiber.app](https://lemonfiber.app),
> rendered from the report of where every unreleased version stands. Then the
> rules** every repository shares:
> [working in the repositories](https://github.com/lemonfiber/spec/blob/main/50-governance/working-in-the-repositories.md)
> and [the rules for agents](https://github.com/lemonfiber/spec/blob/main/50-governance/ai-contributors.md).
> This file holds only what is true of this repository.

## What this repo is

Homepage as a first-party lemonfiber plugin: `plugin.toml` and the adapter that
speaks the `dashboard.show@1` contract in front of Homepage, by writing the two
files it reads, `services.yaml` and `widgets.yaml`, into its configuration
directory. The adapter is the Rust crate at the root, built on
`lemonfiber-adapter-kit` from the core at the commit `Cargo.toml` pins, into the
image the `Dockerfile` builds. `release.yml` publishes that image from every
commit on `main` and from the train's tag. `plugin.toml` runs Homepage and that
adapter beside it. The
[bundle contract](https://github.com/lemonfiber/spec/blob/main/20-architecture/contracts/bundle.md)
says how the catalogue pins this repository and how lemonfiber carries it.

## The rules you cannot break

- **The plugin id is the service's id** (`ARCH-R266`). The container, the
  configuration directory, the settings and the journal keep their names.
- **Two services: `homepage` and `homepage-adapter`.** `homepage` speaks no
  contract and declares `replaces` naming its own id, which is how it takes the
  id, the port and the hostname the bundle reserves for it (`ARCH-R222`).
  `homepage-adapter` fronts it, speaks `dashboard.show@1`, provides
  `dashboard.show` and shares Homepage's configuration directory at `/homepage`
  (`ARCH-R339`).
- **Its proofs' recordings sit under `recordings/homepage/`**, and its starting
  files under `config/homepage/` (`ARCH-R271`).
- **`conformance/` holds one recording per case of `dashboard.show@1`**, written
  by `lemonfiber plugin record .` at the commit `targets.toml` names, the adapter
  at the digest `plugin.toml` pins in front of Homepage at its pinned digest.
  Re-record whenever the adapter's answers or Homepage's digest change.
  `tests/conformance.rs` reads every recording with the core's own client.
- **Every service declares `reaches` and `asks_for`** (`ARCH-R270`).
- **No Python here.** The reader is spec's `first-party-plugin` workflow's.
- **No `unwrap`, `expect`, `panic` or indexing** outside tests. The lints in
  `Cargo.toml` are the core's.
- **Every file in Homepage's configuration directory is reached through
  `lemonfiber_adapter_kit::shared`**, so a link, a hardlink or a file that is not
  plain, which Homepage could plant there, is refused by name or replaced and
  never written through.
- **Every value is held to its grammar before anything is written**: an address
  is `http` or `https`, an icon a plain name or such an address, and text carries
  no control character and nothing Homepage expands (`{{`, `$`).
- **A file changed by hand is never written over.** One that is neither the file
  last written nor the one this plugin ships is named in the refusal.
- **The core's crates move with the train.** The `rev` in `Cargo.toml` is the
  core commit `targets.toml` names, the same for every `lemonfiber-*` crate.
- **Moving Homepage's digest** means recording every response again against the
  new image in the same change, moving `RELEASE` and `DIGEST` in
  `src/adapter.rs` with it, and committing the `proofs.json` the checks write.
  `tests/manifest.rs` fails while the adapter's `DIGEST` and the image the
  manifest runs differ.
