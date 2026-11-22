---
title: Co Server
sdk: docker
app_port: 7860
---

# Skypeline

Ultra-lightweight, Skype-inspired desktop chat, voice calling, and screen-sharing app in **Rust + C++**. Messages persist in Neon Postgres; voice and screen frames are **100% server-relayed (non-P2P)** so user IP addresses are never exposed.

## Hosted service

The desktop client connects only to the hosted relay:

`https://skypeline-co.onrender.com`

There is no local-server selector and no Hugging Face endpoint. Sign-up is handled by Neon Auth (Managed Better Auth):

1. Create an account with email, username, and password.
2. Neon Auth emails a six-digit confirmation code.
3. Enter that code in the client to verify the email and sign in.
4. Returning users sign in with email and password.

Verification is required; unverified accounts cannot obtain a Skypeline session.

## Architecture

- `shared/` — Shared REST & WebSocket protocol types.
- `server/` — Centralized `axum` + WebSocket relay hub with `sqlx` (Postgres + Rustls TLS). Neon Auth verifies identities; the server maps each Neon user UUID to Skypeline's integer user IDs. Friendships, private DMs, and compact avatars live in Postgres. Presence snapshots are personalized so strangers never receive a global user dump.
- `client/` — Native `eframe`/`egui` desktop client with thread-isolated `cpal` + Opus voice, server-relayed 720p30 screen sharing, a dedicated voice/screen stage, and a C++17 native audio/alert bridge (`client/cpp/`).
- `neon.ts` — Declares the Postgres + Neon Auth services for the production branch.

## Deployment

The relay hub is intentionally in-memory, so the server must run as a **single instance**. Do not enable autoscaling.

`render.yaml` describes the single-instance Docker web service. Deploy from the GitHub mirror `https://github.com/skibidiboydawg/skypeline`; Render does not accept Hugging Face Space URLs as repositories.

The server binds `0.0.0.0:$PORT` and serves `GET /` and `GET /health`. The client uses the hosted Render URL above. Private DMs are stored as `dm` channels in Postgres and require an accepted friendship; the Windows client is a GUI-subsystem executable, so opening it never spawns a terminal window.

Friends are added by exact username or email. Incoming requests can be accepted or declined, and only accepted friends appear online or can start a private text/voice/screen session. Profile images are resized to 128x128 and stored as sub-15 KB JPEG bytes (`users.avatar_data`) with disk + memory caching in the client.

The free Render service sleeps after ~15 minutes of inactivity; the first connection can take about a minute to wake it. The client retries with exponential backoff and sends a no-op keepalive every 45 seconds so staying signed in keeps the relay awake. Voice and screen captures survive a reconnect and are re-registered automatically.

## Windows packaging

Run `powershell -NoProfile -ExecutionPolicy Bypass -File installer\build.ps1` to produce:

- `dist/Skypeline-<version>-Portable.zip` — portable folder/zip
- `dist/Skypeline-<version>-Setup.exe` — NSIS installer with Start Menu/desktop shortcuts and uninstall support

Build artifacts live under `dist/` and are git-ignored. Secrets in `.env`/`.env.*` are also git-ignored.

## Linux

On Ubuntu/Mint/Debian or Arch, run `bash linux/build-and-install.sh` (add `--no-install` to skip copying the binary to `/usr/local/bin`). The script installs ALSA, GTK, OpenSSL, X11/Wayland and build dependencies, then produces `./target/release/client`. The C++ bridge gates Windows-only APIs and falls back to `notify-send` plus a lightweight audio chime on Linux. `.github/workflows/linux-build.yml` checks the workspace and release build on every push and pull request.
