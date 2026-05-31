# CapLLM

CapLLM is a multi-provider LLM gateway with a Rust backend and a Next.js admin
dashboard. The gateway exposes an OpenAI-compatible chat completions endpoint
and translates requests to supported upstream providers such as Anthropic and
Gemini.

## What This Project Includes

- Rust workspace for the gateway runtime, provider translation, Redis-backed
  controls, proxying, and security logic.
- OpenAI-style `POST /v1/chat/completions` API for non-streaming and streaming
  chat completions.
- Provider adapters for Anthropic Messages API and Google Gemini GenerateContent
  API.
- Security controls including prompt-injection checks, DLP masking,
  response rehydration, and zero-data-retention header handling where supported.
- Redis-backed tenant virtual keys, token-aware rate limits, query caching, and
  loop detection.
- Next.js dashboard for login, organization management, virtual key management,
  guardrails, billing placeholders, and profile/settings pages.
- Docker Compose setup for running the gateway with Redis and Prometheus.

## Repository Layout

```text
.
├── crates/
│   ├── capllm-core/       # Shared config, errors, and gateway types
│   ├── capllm-proxy/      # HTTP forwarding and SSE streaming helpers
│   ├── capllm-redis/      # Redis cache, tenant store, rate limit, loop breaker
│   ├── capllm-security/   # DLP, prompt injection, vault, and ZDR logic
│   ├── capllm-server/     # Axum HTTP server and gateway handler
│   └── capllm-translate/  # Anthropic and Gemini request/response translation
├── dashboard/             # Next.js admin dashboard with Prisma
├── docker-compose.yml     # Gateway, Redis/Valkey, and Prometheus services
├── Dockerfile             # Rust gateway container build
└── prometheus.yml         # Prometheus scrape configuration
```

## Prerequisites

- Rust 1.75 or newer
- Node.js and npm for the dashboard
- Docker and Docker Compose for containerized local services
- Redis or Valkey if using virtual keys, rate limits, caching, or loop detection

## Backend Usage

Run the gateway locally:

```bash
cargo run -p capllm-server
```

Run tests:

```bash
cargo test
```

Start the Docker Compose stack:

```bash
docker compose up --build
```

The gateway listens on port `3000` by default. Key environment variables:

```text
GATEWAY_PORT=3000
REDIS_URL=redis://127.0.0.1:6379
ANTHROPIC_BASE_URL=https://api.anthropic.com
GEMINI_BASE_URL=https://generativelanguage.googleapis.com
```

## Gateway Request Example

```bash
curl http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <provider-or-virtual-key>" \
  -H "X-Gateway-Provider: anthropic" \
  -d '{
    "model": "claude-3-5-sonnet-latest",
    "messages": [
      { "role": "user", "content": "Explain CapLLM in one sentence." }
    ]
  }'
```

Use `X-Gateway-Provider: gemini` for Gemini-backed requests.

## Dashboard Usage

Install and run the dashboard:

```bash
cd dashboard
npm install
npm run dev
```

The dashboard uses Prisma. Configure local environment variables in
`dashboard/.env` or `dashboard/.env.local`; these files are intentionally not
committed.

## Notes

Generated files, local databases, environment files, build outputs, Redis dumps,
and dependency folders are excluded from Git. Commit source code, lockfiles,
configuration, and documentation only.
