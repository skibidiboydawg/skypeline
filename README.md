---
title: Co Server
sdk: docker
app_port: 7860
---

# Skypeline

Ultra-lightweight, Skype-inspired desktop chat, voice calling, and screen-sharing app in **Rust + C++** with **Aiven PostgreSQL** persistence and **100% server-relayed (NON-P2P)** voice & screen sharing so user IP addresses are never exposed.

## Architecture
- `shared/` — Shared REST & WebSocket protocol types.
- `server/` — Centralized `axum` + WebSocket relay hub + `sqlx` (PostgreSQL + Rustls TLS) backend.
- `client/` — Native `eframe`/`egui` desktop client with `cpal` + Opus voice, 720p30 server-relayed screen sharing, and a C++17 native audio/alert bridge (`client/cpp/`).

## Local Development
1. Create a local `.env` file (git-ignored) containing:
   ```env
   DATABASE_URL=postgres://user:pass@host:port/defaultdb?sslmode=require
   JWT_SECRET=<a long random secret, 32+ bytes>
   PORT=7860
   ```
2. Start the server (binds `0.0.0.0:7860`, auto-runs migrations):
   ```bash
   cargo run -p skypeline-server
   ```
3. Launch the desktop client:
   ```bash
   cargo run -p skypeline-client
   ```
4. In the client **Settings**, set **Server URL** to the local endpoint `http://127.0.0.1:7860`.

The client ships with the default Server URL `https://rezixyzz-co.hf.space` and can be switched at any time in **Settings** to `http://127.0.0.1:7860` for local testing.

## Deploying to Hugging Face Spaces
1. Create a new **Docker** Space on Hugging Face.
2. Add `DATABASE_URL` (your Aiven PostgreSQL `?sslmode=require` URI) and `JWT_SECRET` in **Space Settings → Variables and secrets**.
3. `git push` this repo to your Space remote. The server binds `0.0.0.0:$PORT` (default `7860`) and serves `GET /`.
4. In the desktop client **Settings**, set **Server URL** to `https://rezixyzz-co.hf.space` (the default).

## Git remotes
- Hugging Face Space: `https://huggingface.co/spaces/Rezixyzz/Co`
- Push with `git push hf main` after adding the `hf` remote.
