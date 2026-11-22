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

The client ships with the default Server URL `https://skypeline-co.onrender.com` and can be switched at any time in **Settings** to `http://127.0.0.1:7860` for local testing.

## Deploying to Render + Neon
The relay hub is intentionally in-memory, so the server must run as a **single instance**. Do not enable autoscaling.

1. Create a free PostgreSQL database at [Neon](https://neon.tech). Copy the connection string with `?sslmode=require`.
2. In Render, create a new **Blueprint** from this repository (or a public Git URL pointing at it). `render.yaml` provisions one free Docker web service named `skypeline-co`.
3. When prompted, set:
   - `DATABASE_URL` — the Neon connection string from step 1.
   - `JWT_SECRET` — a long random value, e.g. `openssl rand -hex 32`.
4. Render exposes the service at `https://skypeline-co.onrender.com`. The app binds `0.0.0.0:$PORT` (Render sets `10000`) and serves `GET /` and `GET /health`.
5. In the desktop client **Settings**, set **Server URL** to `https://skypeline-co.onrender.com` (the default). If that Render subdomain is already taken, use the URL Render assigns and update it in Settings.

The free Render service sleeps after ~15 minutes of inactivity; the first connection can take about a minute to wake it. The client now retries with exponential backoff and sends a no-op keepalive every 45 seconds, so staying signed in keeps the relay awake. Voice and screen captures also survive a reconnect and are re-registered automatically.

## Git remotes
- Hugging Face Space (legacy, currently paused on the free CPU quota): `https://huggingface.co/spaces/Rezixyzz/Co`
- Render deploys from the same repository via `render.yaml`.
