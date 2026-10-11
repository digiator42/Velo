# Logs-Soc: Hybrid Log Aggregator & Client-Side Security Threat Intelligence

_A Hybrid Log Aggregator & Client-Side Security Threat Intelligence Platform_

## The Unified Concept

Logs-Soc operates in two complementary modes within one full-stack application:

1. **Real-Time Streaming Mode (Server-Driven):** External systems stream live application telemetry into GritShield. GritShield filters, tags, and pushes logs over Server-Sent Events (SSE) or WebSockets to the veloasm dashboard in real time.

2. **Privacy-Preserving Heavy Analysis Mode (Client-Driven):** Security analysts drag-and-drop multi-gigabyte historical log dumps (e.g., CloudTrail, Nginx, PCAPs) directly into the browser. veloasm parses and analyzes logs locally inside Web Workers using compiled Rust rule engines without uploading sensitive raw data. Only extracted, verified threat alerts and malicious IPs are synced back to GritShield's threat database.

## System Architecture

`
[EXTERNAL SOURCES] -> GritShield (ingest, filter, ring buffer, SSE/WS)
                  -> veloasm WASM Frontend (reactive dashboard)
                  -> Web Worker (chunked parsing: regex, Shannon entropy, sliding-window anomaly)
                     -> flags -> POST /api/v1/threats -> GritShield DB + MCP
`

## Phases

### Phase 1: Ingestion & SSE (Backend)
- POST /api/v1/logs (single/batch JSON)
- Async middleware (on_request auth/size/rate-limit; on_response timing/headers)
- SSE /api/v1/stream via ResponseBody::Stream + sse::pump
- In-memory ring buffer (last 10k events)

### Phase 2: WASM Parsing & Threat Detection (Frontend)
- Drag-and-drop bulk ingestion (multi-GB)
- Web Worker offload with ReadableStream + chunk transfer
- Detection: traversal ../, SQLi OR 1=1, XSS <script>, scanner patterns; Shannon entropy; per-IP sliding-window rate anomaly

### Phase 3: Threat Intel & MCP (Backend)
- POST /api/v1/threats (malicious IP, level, payload, ts)
- MCP tools/prompts: get_active_threats, add_ip_blacklist, summarize_incident

### Phase 4: Reactive SOC Dashboard (Frontend)
- Live log terminal (auto-scroll, INFO/WARN/ERROR/CRITICAL)
- Threat alert cards (high entropy/signatures)
- Mode toggle (Live vs Local File Analysis), filters (level/route/IP), metrics (logs/sec, threats, peak bandwidth)

## Framework Verification Matrix
| Framework Feature | Component | Success Criteria |
|---|---|---|
| GritShield Async Middleware | rate limiting + security headers | non-blocking, clean short-circuit |
| GritShield SSE Pumping | /api/v1/stream | sustained push, no socket FD leaks |
| GritShield MCP Server | tools/prompts | AI agents query/mutate |
| veloasm Web Worker WASM | multi-threaded parser | 1M lines (~500MB), no main-thread drops |
| veloasm Reactive DOM | live feed | 100+ events/sec smooth |

