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
1. Sign in to Neon and pull the branch env into git-ignored `.env.local`:
   ```bash
   neon login
   neon env pull
   ```
   This writes `DATABASE_URL` (pooled), `DATABASE_URL_UNPOOLED` (direct, used for migrations), and branch metadata. Add your own `JWT_SECRET` and `PORT` to that file; see `env.example`.
2. Start the server (binds `0.0.0.0:7860`, auto-runs migrations):
   ```bash
   npm run start
   ```
3. Launch the desktop client:
   ```bash
   cargo run -p skypeline-client
   ```
4. In the client **Settings**, set **Server URL** to the local endpoint `http://127.0.0.1:7860`.

The client ships with the default Server URL `https://skypeline-co.onrender.com` and can be switched at any time in **Settings** to `http://127.0.0.1:7860` for local testing.

## Deploying to Render + Neon
The relay hub is intentionally in-memory, so the server must run as a **single instance**. Do not enable autoscaling.

1. Create a free PostgreSQL database at [Neon](https://neon.tech). In the console, click **Connect**, choose the `production` branch, and copy the **pooled** connection string (`-pooler` in the hostname).
2. In Render, create a new **Web Service** from `https://github.com/skibidiboydawg/skypeline`. `render.yaml` describes the same single-instance Docker setup; the GitHub mirror exists because Render does not accept Hugging Face Space URLs.
3. When prompted, set:
   - `DATABASE_URL` — the pooled Neon connection string from step 1. The server uses `DATABASE_URL_UNPOOLED` for DDL when Neon provides it locally, but Render only needs `DATABASE_URL`.
   - `JWT_SECRET` — a long random value, e.g. `openssl rand -hex 32`.
4. Render exposes the service at `https://skypeline-co.onrender.com`. The app binds `0.0.0.0:$PORT` (Render sets `10000`) and serves `GET /` and `GET /health`.
5. In the desktop client **Settings**, set **Server URL** to `https://skypeline-co.onrender.com` (the default). If that Render subdomain is already taken, use the URL Render assigns and update it in Settings.

The free Render service sleeps after ~15 minutes of inactivity; the first connection can take about a minute to wake it. The client now retries with exponential backoff and sends a no-op keepalive every 45 seconds, so staying signed in keeps the relay awake. Voice and screen captures also survive a reconnect and are re-registered automatically.

## Git remotes
- Hugging Face Space (legacy, currently paused on the free CPU quota): `https://huggingface.co/spaces/Rezixyzz/Co`
- Render deploys from the same repository via `render.yaml`.
