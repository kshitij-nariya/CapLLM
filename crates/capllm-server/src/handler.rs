//! Route handler for `POST /v1/chat/completions`.
//!
//! Pipeline: Auth → Rate Limit → **Injection Guard** → **DLP Mask** →
//! Translate → **ZDR Headers** → Forward → **Re-hydrate** → Respond.
//!
//! **Phase 2**: Virtual key auth, rate limiting, query caching.
//! **Phase 3**: DLP masking, prompt injection defense, ZDR enforcement.

use std::convert::Infallible;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Response};
use axum::Json;
use capllm_core::{ChatCompletionRequest, GatewayError, Provider, TenantMeta};
use capllm_proxy::into_openai_sse_stream;
use capllm_redis::{LoopBreaker, QueryCache, RateLimiter, TenantStore};
use capllm_security::{DlpEngine, InjectionGuard, ZdrEnforcer};
use capllm_translate::TranslationEngine;
use futures::stream::StreamExt;
use tracing::instrument;

use crate::state::AppState;
use crate::telemetry::metrics;
use std::time::Instant;

/// `POST /v1/chat/completions`
#[allow(clippy::too_many_lines)]
#[instrument(skip_all, fields(provider, stream))]
pub async fn chat_completions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ChatCompletionRequest>,
) -> Result<Response, GatewayError> {
    // ── 1. Extract auth token ────────────────────────────────────────────
    let auth_value = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.strip_prefix("Bearer ").unwrap_or(v))
        .ok_or_else(|| GatewayError::MissingHeader("Authorization".to_owned()))?;

    // ── 2. Virtual key or passthrough? ───────────────────────────────────
    let (api_key, tenant, provider) = if auth_value.starts_with("gw-") {
        let redis = state
            .redis
            .as_ref()
            .ok_or_else(|| GatewayError::ConfigError("Redis not configured for virtual keys".to_owned()))?;

        let meta = TenantStore::resolve(redis, auth_value).await?;
        let provider: Provider = meta.provider.parse()?;

        let estimated_tokens = RateLimiter::estimate_tokens(&request);
        RateLimiter::check_and_record(
            redis,
            &meta.team,
            estimated_tokens,
            meta.tpm_limit,
            meta.spend_cap,
        )
        .await?;

        let prompt_hash = LoopBreaker::compute_hash(&request);
        LoopBreaker::check(redis, &meta.team, &prompt_hash).await?;

        if !request.stream {
            let cache_key = QueryCache::cache_key(&request);
            if let Some(cached) = QueryCache::get(redis, &cache_key).await? {
                tracing::info!("returning cached response");
                return Ok(axum::response::Html(cached).into_response());
            }
        }

        let key = meta.vendor_key.clone();
        (key, Some(meta), provider)
    } else {
        let provider_str = headers
            .get("x-gateway-provider")
            .ok_or_else(|| GatewayError::MissingHeader("X-Gateway-Provider".to_owned()))?
            .to_str()
            .map_err(|_| GatewayError::InvalidProvider("non-ASCII header value".to_owned()))?;

        let provider: Provider = provider_str.parse()?;
        (auth_value.to_owned(), None, provider)
    };

    tracing::Span::current().record("provider", provider.to_string().as_str());
    tracing::Span::current().record("stream", request.stream);

    // ── 3. SECURITY: Prompt injection guard ──────────────────────────────
    InjectionGuard::check(&request)?;

    // ── 4. SECURITY: DLP scan & mask ─────────────────────────────────────
    let sanitized_request = state.dlp.scan_and_mask(&request, &state.vault);

    // ── 5. Resolve base URL ──────────────────────────────────────────────
    let base_url = match provider {
        Provider::Anthropic => &state.config.anthropic_base_url,
        Provider::Gemini => &state.config.gemini_base_url,
    };

    // ── 6. Translate request ─────────────────────────────────────────────
    let model = sanitized_request.model.clone();
    let is_stream = sanitized_request.stream;
    let cache_key = if !is_stream && state.redis.is_some() {
        Some(QueryCache::cache_key(&request))
    } else {
        None
    };

    let (url, mut body, mut fwd_headers) =
        TranslationEngine::translate_request(provider, &sanitized_request, &api_key, base_url)?;

    // ── 7. SECURITY: ZDR enforcement ─────────────────────────────────────
    ZdrEnforcer::apply_headers(provider, &mut fwd_headers);
    ZdrEnforcer::apply_body(provider, &mut body);

    tracing::info!(url = %url, "forwarding request to upstream");

    let start_time = Instant::now();
    let team_str = tenant.as_ref().map_or("unknown", |t| &t.team).to_owned();
    
    // ── 8. Forward & respond ─────────────────────────────────────────────
    let res = if is_stream {
        handle_streaming(&state, &url, &body, fwd_headers.clone(), provider, model.clone()).await
    } else {
        handle_non_streaming(
            &state, &url, &body, fwd_headers.clone(), provider, &model,
            cache_key.as_deref(), tenant.as_ref(),
        ).await
    };

    match res {
        Ok(response) => {
            metrics::record_request(&provider.to_string(), response.status().as_u16(), &team_str);
            metrics::record_latency(&provider.to_string(), start_time.elapsed());
            Ok(response)
        }
        Err(e) if is_failover_error(&e) => {
            tracing::warn!(error = %e, "primary provider failed, attempting failover");
            
            let fallback_provider = match provider {
                Provider::Anthropic => Provider::Gemini,
                Provider::Gemini => Provider::Anthropic,
            };
            
            let fallback_url = match fallback_provider {
                Provider::Anthropic => &state.config.anthropic_base_url,
                Provider::Gemini => &state.config.gemini_base_url,
            };

            let (f_url, mut f_body, mut f_headers) = TranslationEngine::translate_request(
                fallback_provider,
                &sanitized_request,
                &api_key,
                fallback_url,
            )?;
            ZdrEnforcer::apply_headers(fallback_provider, &mut f_headers);
            ZdrEnforcer::apply_body(fallback_provider, &mut f_body);

            let fallback_res = if is_stream {
                handle_streaming(&state, &f_url, &f_body, f_headers, fallback_provider, model).await
            } else {
                handle_non_streaming(
                    &state, &f_url, &f_body, f_headers, fallback_provider, &model,
                    cache_key.as_deref(), tenant.as_ref(),
                ).await
            };

            match fallback_res {
                Ok(response) => {
                    metrics::record_request(&fallback_provider.to_string(), response.status().as_u16(), &team_str);
                    metrics::record_latency(&fallback_provider.to_string(), start_time.elapsed());
                    Ok(response)
                }
                Err(fallback_err) => {
                    tracing::error!(error = %fallback_err, "fallback provider also failed");
                    metrics::record_request(&fallback_provider.to_string(), 500, &team_str);
                    Err(fallback_err)
                }
            }
        }
        Err(e) => {
            metrics::record_request(&provider.to_string(), 500, &team_str);
            Err(e)
        }
    }
}

const fn is_failover_error(err: &GatewayError) -> bool {
    matches!(err, GatewayError::UpstreamError(_) | GatewayError::HttpClient(_))
}

/// Stream the upstream SSE response back as OpenAI-format SSE events.
async fn handle_streaming(
    state: &AppState,
    url: &str,
    body: &serde_json::Value,
    headers: HeaderMap,
    provider: Provider,
    model: String,
) -> Result<Response, GatewayError> {
    let response = state.proxy.forward_streaming(url, body, headers).await?;
    let chunk_stream = into_openai_sse_stream(response, provider, model);

    let sse_stream = chunk_stream.map(|result| -> Result<Event, Infallible> {
        match result {
            Ok(chunk) => {
                let json = serde_json::to_string(&chunk).unwrap_or_default();
                Ok(Event::default().data(json))
            }
            Err(e) => {
                tracing::error!(error = %e, "SSE translation error");
                Ok(Event::default().data(format!("[ERROR] {e}")))
            }
        }
    });

    let done_stream = futures::stream::once(async {
        Ok::<Event, Infallible>(Event::default().data("[DONE]"))
    });

    let full_stream = sse_stream.chain(done_stream);

    Ok(Sse::new(full_stream)
        .keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(std::time::Duration::from_secs(15))
                .text("keep-alive"),
        )
        .into_response())
}

/// Forward a non-streaming request, translate the response, re-hydrate DLP
/// placeholders, cache it, and return JSON.
#[allow(clippy::too_many_arguments)]
async fn handle_non_streaming(
    state: &AppState,
    url: &str,
    body: &serde_json::Value,
    headers: HeaderMap,
    provider: Provider,
    model: &str,
    cache_key: Option<&str>,
    _tenant: Option<&TenantMeta>,
) -> Result<Response, GatewayError> {
    let response_body = state.proxy.forward(url, body, headers).await?;
    let mut translated = TranslationEngine::translate_response(provider, &response_body, model)?;

    // ── SECURITY: Re-hydrate DLP placeholders in response ────────────────
    for choice in &mut translated.choices {
        choice.message.content =
            DlpEngine::rehydrate(&choice.message.content, &state.vault);
    }

    // Cache the response if Redis is available
    if let (Some(redis), Some(key)) = (state.redis.as_ref(), cache_key) {
        let json_str = serde_json::to_string(&translated).unwrap_or_default();
        let redis = redis.clone();
        let key = key.to_owned();
        tokio::spawn(async move {
            if let Err(e) = QueryCache::set(&redis, &key, &json_str, None).await {
                tracing::warn!(error = %e, "failed to cache response");
            }
        });
    }

    Ok(Json(translated).into_response())
}

// ─── Health Check ────────────────────────────────────────────────────────────

/// `GET /health`
pub async fn health() -> &'static str {
    "ok"
}

/// `GET /metrics`
pub async fn metrics(State(state): State<AppState>) -> String {
    state.telemetry.export_metrics()
}

// ─── Dashboard ───────────────────────────────────────────────────────────────

/// `GET /` — serves the embedded HTML dashboard.
pub async fn dashboard() -> Response {
    axum::response::Html(include_str!("../static/index.html")).into_response()
}
