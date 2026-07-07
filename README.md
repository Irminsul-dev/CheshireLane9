![cheshire-banner](assets/static/cheshire-banner.png)

# CheshireLane9

CheshireLane9 is a server emulator for an anime fleet game client Version 9.x.

It is an upgrade of the old [CheshireLane](https://github.com/Irminsul-dev/CheshireLane.git) project. The name changed because the client did. The problems, naturally, found new and interesting ways to remain problems.

## Screenshot

![screen shot](assets/static/screen_shot.png)

The repository lives at:

```bash
git clone https://github.com/Irminsul-dev/CheshireLane9.git
cd CheshireLane9
```

## What It Is

CheshireLane9 currently runs the SDK, dispatch, gate, and game handling in one Rust binary. There is no heroic service mesh here; one executable is already enough paperwork.

The implementation uses local protobuf definitions under `crates/proto`, game data from `assets/game`, and a SQLite database by default. Configuration is generated from `src/config.default.toml` into `config.toml` on first run.

We know you all want the damn proto files directly, so this time they are open-sourced too; spare yourselves the miserable little scripts people keep writing to extract them.

Default ports:

- SDK HTTP: `21080`
- SDK HTTPS: `21443`
- Dispatch: `21180`
- Gate: `21280`

## Requirements

To build and run the server:

- Rust toolchain
- The game data already present under `assets/game`
- A client compatible with Version 9.x

For client redirection, depending on your device setup, you may also need:

- Root access, commonly through [Magisk](https://github.com/topjohnwu/Magisk)
- User CA trust support, for example [NVISOsecurity/AlwaysTrustUserCerts](https://github.com/NVISOsecurity/AlwaysTrustUserCerts)
- `mitmproxy`, if you use the SDK redirect script
- `iptables`, if you use the game redirect scripts

## Build And Run

```bash
cargo run -p cheshire-server
```

The server reads `config.toml` from the working directory. If the file does not exist, it writes the default one. This is convenient, unless you expected configuration to be a spiritual journey.

## Redirect Scripts

Redirect helper scripts live in:

```text
scripts/redirect/
```

### SDK Redirect

Use the mitmproxy addon to send SDK API traffic to the local SDK HTTP server:

```bash
mitmproxy -s scripts/redirect/redirect_sdk.py
```

The script redirects:

- `jp-sdk-api.yostarplat.com`
- `en-sdk-api.yostarplat.com`

to:

```text
http://127.0.0.1:21080
```

Install and trust the mitmproxy certificate on the client. On Android, user certificates may not be trusted by apps by default; Magisk plus AlwaysTrustUserCerts is the boring answer that usually works.

### Game Redirect

On the rooted client shell:

```bash
su
sh scripts/redirect/redirect_game.sh
```

This redirects the known destination IPs for:

- `blhxjploginapi.azurlane.jp`
- `blhxusgate.yo-star.com`

to:

```text
127.0.0.1:21180
```

To remove the rules:

```bash
su
sh scripts/redirect/unredirect_game.sh
```

These scripts use `setenforce` and `iptables`. If your Android build has opinions about either, it will share them loudly.

## Status

This is research software. Some flows work, some flows are placeholders, and some flows are still waiting for more testing to explain what they want when they grow up.

Use it for learning, protocol work, and local experiments. Do not sell it, do not run a public service with it, and do not make the maintainer read legal emails before coffee.
