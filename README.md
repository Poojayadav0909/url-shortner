# url-shortner

A URL shortener hosted on Cloudflare Pages + Supabase (Postgres).

- **Live site:** https://url-shortner.pages.dev
- **Backend storage:** Supabase (table `urls`)

## Setup

### 1. Create the table in Supabase

Open Supabase → SQL Editor → New query, paste the contents of [`sql/1.sql`](sql/1.sql), and run it.

### 2. Deploy to Cloudflare Pages

```bash
# (Node 22 or newer required for wrangler)
export PATH="$HOME/.nvm/versions/node/v22.23.1/bin:$PATH"

# Create the project once
wrangler pages project create url-shortner --production-branch main

# Set the Supabase secrets used by the redirect function
echo "https://zfpkjkbdtvwrjwblvzkd.supabase.co" | wrangler pages secret put SUPABASE_URL --project-name url-shortner
echo "sb_secret_..." | wrangler pages secret put SUPABASE_SERVICE_KEY --project-name url-shortner

# Deploy
wrangler pages deploy
```

## How it works

- `public/index.html` — the frontend. It talks directly to the Supabase REST API
  using the publishable (anon) key to create short links and list stats.
- `functions/[[code]].js` — a Cloudflare Pages Function that handles `/<code>`
  requests: looks up the URL, increments the click counter (service key), and
  redirects the visitor.

## Local Rust backend (optional)

The repo also includes an Axum backend (`cargo run`) that serves the same
frontend and can proxy the API. It is not required for the hosted site.

```bash
cargo run
# open http://localhost:3000
```