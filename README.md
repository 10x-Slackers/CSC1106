# CSC1106 Web Programming Project

Accounting web application with a double-entry ledger built with Rust, Actix Web, Tera templates, SQLite, SeaORM, TailwindCSS, and DaisyUI.

---

## Project Scope

- [`src`](./src)
  - [`src/routes`](./src/routes) Actix Web handlers
  - [`src/models`](./src/models) business logic, validation, accounting workflows, report calculations, shared types, and application errors
  - [`src/entity`](./src/entity) SeaORM table mappings
  - [`src/middleware`](./src/middleware) authentication, role checks, security headers, and Tera filters
  - [`src/pdf`](./src/pdf) PDF rendering
- [`templates`](./templates) Tera server-side rendered pages
  - [`templates/macros`](./templates/macros) reusable compoenents
  - [`templates/partials`](./templates/partials) shared layouts
- [`migration`](./migration) SeaORM database migrations for the SQLite schema and indexes
- [`assets`](./assets) compiled CSS and embedded font files used by the web UI and PDFs

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
- On first run, if no users exist, the server prompts in the terminal to create the initial Admin account
  - There are no hard-coded default login credentials

Optional environment variables:

| Variable       | Default                       | Purpose                  |
| -------------- | ----------------------------- | ------------------------ |
| `HOST`         | `localhost`                   | HTTP bind host           |
| `PORT`         | `8080`                        | HTTP bind port           |
| `DATABASE_URL` | `sqlite://./data.db?mode=rwc` | SQLite database location |
| `SECRET_KEY`   | Random generated key per run  | Session signing key      |

### Production Build and Use

Build the minified frontend assets and optimized Rust binary:

```sh
just build
```

Run the release binary with a stable session secret:

```sh
SECRET_KEY=example-secret-i-love-peter-yau ./target/release/csc1106
```

Set `HOST`, `PORT`, and `DATABASE_URL` as needed. The server runs migrations automatically on startup and serves the app from the configured host and port.

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

4. Install Node dependencies if needed

   ```sh
   npm install
   ```

5. Start the development server

   ```sh
   just dev
   ```

## Developer Tooling

- Dev Containers
  - Standardised developer environment
- Pre-Commit
  - Run linting and formatting for files during git commit
- Cargo / Clippy
  - Rust build and lint tooling
- npm / TailwindCSS / DaisyUI / Prettier
  - Frontend asset build and formatting
