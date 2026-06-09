# CSC1106 Web Programming Project

> [!NOTE]
> WIP, remove this note when project is ready.

Accounting web application with double-entry system built with Rust (Actix-web) and Tera templates (TailwindCSS + DaisyUI for styling).

---

## Project Scope

- [link_to_source](link_to_source)
  - scope_description

## Usage

### Justfile

| Command       | Description                                        |
| ------------- | -------------------------------------------------- |
| `just dev`    | Start both Rust and frontend watchers concurrently |
| `just build`  | Build frontend assets and Rust binary (release)    |
| `just format` | Auto-format Rust and frontend code                 |
| `just lint`   | Run Clippy lint checks                             |

### Server

- Start the server with `just dev`
- Access the app at <http://localhost:8080>
- Default credentials (for now):
  - `admin@example.com:P@ssw0rd` (admin role)
  - `john@example.com:P@ssw0rd` (accountant role)

## Getting Started

### Prerequisites

- [Git](https://github.com/git-guides/install-git) (fully set-up)
- [Docker/Podman](https://docs.docker.com/engine/install/)
- [VS Code](https://code.visualstudio.com/download)
  - [Dev Containers](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers) extension

> [!WARNING]
> Do not use GitHub Desktop! All interactions (files, git, runtime, etc.) should be done through the Dev Container within VS Code.

### Installation

1. Clone the repo

   ```sh
   git clone git@github.com:10x-Slackers/CSC1106.git
   ```

2. Open the repository in VS Code

   ```sh
   code CSC1106/
   ```

3. Click on the "Re-open in Dev Container" prompt

4. Start working!

## Developer Tooling

- Dev Containers
  - Standardised developer environment
- Pre-Commit
  - Run linting and formatting for all files during git commit
