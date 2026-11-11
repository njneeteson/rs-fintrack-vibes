# Deploying FinTrack on a home server

FinTrack is a self-hosted household finance web app (Rust, single binary, SQLite).
This document has everything needed to deploy it on a home server with Docker
Compose behind an existing reverse proxy, **without building on the server**.

## At a glance

| | |
|---|---|
| Image | `ghcr.io/njneeteson/fintrack:latest` (built by GitHub Actions; see below) |
| Image size / footprint | small: distroless base (~25 MB) plus one stripped binary; expect tens of MB of RAM and negligible CPU at household scale (estimates, not measured) |
| Listens on | plain HTTP, port **8080** inside the container (`FINTRACK_BIND`, default `0.0.0.0:8080`) |
| URL prefix | set by `FINTRACK_BASE_PATH`, e.g. `/finances` → `https://miniserver.local/finances` |
| Persistent data | one volume mounted at **`/data`** (SQLite files and archived CSV imports) |
| Runs as | uid/gid **65532** (distroless `nonroot`), so `/data` must be writable by it |
| Health check | `GET <base>/healthz` → `200 ok`; built into the image (`fintrack-server healthcheck`) |
| Outbound network | none (never contacts the internet) |
| TLS | not handled by FinTrack; terminate HTTPS at the reverse proxy |

## Getting the image onto the server

The Rust compile is the only heavy part. Do it elsewhere and ship the finished
image. Pick one option.

### Option A (recommended): pull from GitHub Container Registry

The repository contains `.github/workflows/docker.yml`. Every push to `main`
runs the tests, builds the image and publishes:

- `ghcr.io/njneeteson/fintrack:latest` (latest `main`)
- `ghcr.io/njneeteson/fintrack:0.2.0`, `:0.2` (when you push a tag `v0.2.0`)
- `ghcr.io/njneeteson/fintrack:sha-<commit>` (every build)

One-time setup:

1. Push the repository to GitHub, then check the **Actions** tab for a green "Docker image" run.
2. The package appears under your GitHub profile → **Packages → fintrack**.
   - **Private repo:** the package is private. On the server, log in once with a
     personal access token (classic) that has only the `read:packages` scope:
     `echo <TOKEN> | docker login ghcr.io -u njneeteson --password-stdin`
   - Or make the package public in its settings, so no login is needed.
3. **ARM server (Raspberry Pi etc.)?** The workflow builds `linux/amd64` only.
   Change `PLATFORMS` in the workflow to `linux/amd64,linux/arm64`; the ARM build
   is emulated and slow.

On the server: `docker compose pull && docker compose up -d`.

### Option B: build on your PC, copy the image file

No registry involved. Needs Docker (Docker Desktop on Windows) on the PC.

```sh
# On the PC, in the repository (use --platform linux/arm64 for an ARM server)
docker buildx build --platform linux/amd64 -t fintrack:0.2.0 --load .
docker save fintrack:0.2.0 | gzip > fintrack-0.2.0.tar.gz
scp fintrack-0.2.0.tar.gz user@miniserver.local:~

# On the server
gunzip -c fintrack-0.2.0.tar.gz | docker load
```

Then use `image: fintrack:0.2.0` in the compose file below.

## Compose file for the server

Only this file is needed on the server; no repository checkout.

```yaml
# ~/fintrack/compose.yaml
services:
  fintrack:
    image: ghcr.io/njneeteson/fintrack:latest   # or fintrack:0.2.0 with option B
    container_name: fintrack
    restart: unless-stopped
    environment:
      FINTRACK_BASE_PATH: /finances      # must match the path the proxy forwards
      TZ: America/Vancouver              # "today"/"this month" use this time zone
      FINTRACK_SECURE_COOKIES: "true"    # HTTPS-only cookies; keep true behind an HTTPS proxy
      FINTRACK_SESSION_DAYS: "30"
      # Optional instead of the setup link (remove after first start):
      # FINTRACK_ADMIN_USER: alice
      # FINTRACK_ADMIN_PASSWORD: change-me-please
    volumes:
      - fintrack-data:/data
      # or a host folder (must be owned by 65532: sudo chown -R 65532:65532 ./data)
      # - ./data:/data
    ports:
      - "127.0.0.1:8080:8080"            # only the proxy on this host needs it
    # If the reverse proxy is itself a container, drop `ports:` and join its network:
    # networks: [proxy]

volumes:
  fintrack-data:

# networks:
#   proxy:
#     external: true
```

## Reverse proxy requirements

- Forward `/finances` and everything under it to `http://fintrack:8080`
  (container network) or `http://127.0.0.1:8080` (published port).
- **Do not strip the prefix.** FinTrack expects the full path, `/finances/...`.
- Pass the usual headers: `Host`, `X-Forwarded-For`, `X-Forwarded-Proto`,
  `X-Forwarded-Host`. `X-Forwarded-Proto: https` matters: with secure cookies on,
  the sign-in page warns if the request doesn't look like HTTPS.
- Allow request bodies up to 50 MB (CSV and database uploads), or set
  `FINTRACK_MAX_UPLOAD_MB` to match the proxy's limit.
- No WebSockets, sticky sessions or special timeouts are needed.

Examples:

```caddy
# Caddy: `handle` keeps the prefix (not handle_path)
miniserver.local {
	handle /finances* {
		reverse_proxy fintrack:8080
	}
}
```

```nginx
# nginx: no URI after the upstream, so the full path is passed
location /finances {
    proxy_pass http://127.0.0.1:8080;
    proxy_set_header Host              $host;
    proxy_set_header X-Forwarded-For   $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
    proxy_set_header X-Forwarded-Host  $host;
    client_max_body_size 50m;
}
```

```yaml
# Traefik labels on the fintrack service (no StripPrefix middleware)
labels:
  - traefik.enable=true
  - traefik.http.routers.fintrack.rule=Host(`miniserver.local`) && PathPrefix(`/finances`)
  - traefik.http.routers.fintrack.entrypoints=websecure
  - traefik.http.routers.fintrack.tls=true
  - traefik.http.services.fintrack.loadbalancer.server.port=8080
```

## First start

```sh
cd ~/fintrack
docker compose up -d
docker compose logs fintrack
```

The log contains a one-time setup link like
`/finances/setup?token=…`. Open it through the proxy
(`https://miniserver.local/finances/setup?token=…`) and create the admin
account. The link stops working once an admin exists. Alternatively:

```sh
docker compose exec fintrack fintrack-server admin create alice   # prints a temporary password
```

Then, in the web UI: **Manage households → New household** (optionally with demo
data), **Manage users → Add a user**, and the **Access** tab to grant each person
*View only* or *Can edit* per household.

## All settings

| Variable | Default | Meaning |
|---|---|---|
| `FINTRACK_BASE_PATH` | `/` | URL prefix, e.g. `/finances` |
| `FINTRACK_BIND` | `0.0.0.0:8080` | listen address inside the container |
| `FINTRACK_DATA_DIR` | `/data` | data location (keep the default in Docker) |
| `FINTRACK_SECURE_COOKIES` | `true` | cookies only over HTTPS; `false` only for plain-HTTP setups |
| `FINTRACK_SESSION_DAYS` | `30` | "Keep me signed in" duration |
| `FINTRACK_MAX_UPLOAD_MB` | `50` | largest upload |
| `FINTRACK_ADMIN_USER` / `FINTRACK_ADMIN_PASSWORD` | unset | create this admin at start-up if none exists |
| `TZ` | `UTC` | time zone |
| `RUST_LOG` | `info` | log level |

## Data, backups and restore

```text
/data
├── auth.sqlite              users, households, access, sessions
└── households/<name>/       one SQLite database folder per household
    ├── fintrack.sqlite
    └── imports/             copies of imported CSVs
```

- **While running:** in the web UI, *Manage households → Download backup* gives a
  consistent `.sqlite` snapshot per household.
- **Whole volume:** stop first for a consistent copy:
  ```sh
  docker compose stop fintrack
  docker run --rm -v fintrack_fintrack-data:/data -v "$PWD":/backup alpine \
    tar czf /backup/fintrack-data-$(date +%F).tgz -C /data .
  docker compose start fintrack
  ```
  (The volume name is `<project>_fintrack-data`; check with `docker volume ls`.)
- **Restore a household:** *Manage households → Restore or migrate a database*
  (upload a `fintrack.sqlite`).
- Deleted households are moved to `/data/households/.trash/`, not destroyed.

## Updating

```sh
docker compose pull          # option A; for option B, `docker load` the new tarball
docker compose up -d
docker image prune -f        # remove the old image
```

Database schemas upgrade automatically on start. To pin a version, use a tag
such as `ghcr.io/njneeteson/fintrack:0.2.0` instead of `latest`.

## Maintenance commands

```sh
docker compose exec fintrack fintrack-server admin list
docker compose exec fintrack fintrack-server admin reset-password <name>   # new temporary password
docker compose exec fintrack fintrack-server healthcheck                   # exit 0 = healthy
docker compose logs -f fintrack
```

## Troubleshooting

| Symptom | Cause / fix |
|---|---|
| Sign-in succeeds but returns to the sign-in page | Browser isn't on HTTPS, or the proxy doesn't send `X-Forwarded-Proto: https`. Fix the proxy, or set `FINTRACK_SECURE_COOKIES=false` for plain HTTP. |
| Every page is "Not found" | The proxy strips `/finances`, or `FINTRACK_BASE_PATH` doesn't match the proxy path. |
| Container exits with a permission error on `/data` | A bind-mounted folder isn't writable by uid 65532: `sudo chown -R 65532:65532 ./data`. |
| `docker pull` says unauthorized | Private GHCR package: `docker login ghcr.io` with a `read:packages` token, or make the package public. |
| `exec format error` | Image built for the wrong CPU architecture; build or publish `linux/arm64` for ARM servers. |
| Upload fails for large files | Raise the proxy's body size limit and `FINTRACK_MAX_UPLOAD_MB`. |
