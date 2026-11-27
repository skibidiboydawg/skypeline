# Wireline

A lightweight desktop app for private messaging, voice calls, and 720p screen sharing.

- **Messaging:** Direct messages, typing indicators, message editing/deletion, and friend requests.
- **Voice & Screen Sharing:** Low-latency Opus voice calls and 720p30 screen sharing relayed through the server so user IP addresses are never exposed.
- **Cross-Platform:** Native desktop client for Windows and Linux (Arch & Linux Mint/Ubuntu).

## Install

### Windows
- **Web Installer:** Run `Wireline-WebSetup.exe` from [Releases](https://github.com/skibidiboydawg/skypeline/releases/latest).
- **Offline Installer:** Run `Wireline-Setup.exe` from [Releases](https://github.com/skibidiboydawg/skypeline/releases/latest).

### Linux (Arch / Linux Mint / Ubuntu)
```bash
curl -fsSL https://raw.githubusercontent.com/skibidiboydawg/skypeline/main/linux/install.sh | bash
```
Or build from source:
```bash
git clone https://github.com/skibidiboydawg/skypeline.git
cd skypeline
bash linux/build-and-install.sh
```
