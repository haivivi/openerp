# OpenERP

Modular, embeddable enterprise resource planning services built in Rust.

## What is OpenERP?

OpenERP is a collection of lightweight, independently deployable services for common enterprise operations. Each service is a single binary with embedded storage — no external databases required.

**Services:**

| Service | Description |
|---------|-------------|
| **auth** | Federated authentication (OAuth: Feishu, GitHub, Google, ...), hierarchical groups, roles, and policy-based access control |
| **pms** | Product & manufacturing management — devices, SN encoding, production batches, licensing, firmware, device logs |

## Design Principles

- **Single binary** — Each service compiles to one executable. Deploy anywhere.
- **Embedded storage** — KV (redb), SQL (SQLite), full-text search (tantivy), log engine. No PostgreSQL, no Redis, no Elasticsearch required.
- **File + DB hybrid** — Static configuration loaded from files (YAML/JSON), dynamic data in embedded DB. Same API, transparent to callers. File-sourced data is read-only.
- **Multi-instance** — Run multiple instances with different data directories for different tenants/environments.
- **Standard REST API** — Every resource supports CRUD. Read-only resources return 403 on writes.
- **Shared foundation** — Common storage traits (`KVStore`, `SQLStore`, `SearchEngine`, `BlobStore`, `LogStore`) shared across all services.

## Architecture

```
┌──────────────────────────────────────────────────────┐
│  Service (auth / pms / ...)                           │
│                                                       │
│  ┌─────────────────────────────────────────────────┐ │
│  │  HTTP API (axum)                                 │ │
│  └──────────────────────┬──────────────────────────┘ │
│                         │                             │
│  ┌──────────────────────▼──────────────────────────┐ │
│  │  Service Logic                                   │ │
│  └──────────────────────┬──────────────────────────┘ │
│                         │                             │
│  ┌──────────────────────▼──────────────────────────┐ │
│  │  Storage Layer (shared traits)                   │ │
│  │                                                  │ │
│  │  KVStore ─── File overlay (readonly) + redb (rw) │ │
│  │  SQLStore ── SQLite                              │ │
│  │  SearchEngine ── tantivy                         │ │
│  │  BlobStore ── filesystem                         │ │
│  │  LogStore ── WAL + zstd + label index            │ │
│  └──────────────────────────────────────────────────┘ │
│                                                       │
│  --data-dir=/path/to/config  --db=/path/to/data.redb  │
└──────────────────────────────────────────────────────┘
```

## Services

### Auth

Federated identity + hierarchical groups + policy-based access control.

**Resources:** User, Group, Provider, Role, Policy

**Key features:**
- OAuth login (Feishu, GitHub, Google, custom)
- Hierarchical groups with nesting (groups can contain users, sub-groups, or external references like Feishu departments / GitHub teams)
- Policy as (who, what?, how, time?) tuples — flexible ACL
- Group expansion with TTL-based lazy caching
- Cycle detection on group nesting (DFS)
- JWT issuance with embedded groups and roles
- Permission check endpoint for other services

### PMS

Product & manufacturing management system.

**Resources:** Model, SNConfig, Batch, Device, License, LicenseType, Firmware, DeviceLog

**Key features:**
- SN encoding engine — configurable multi-segment encoding (80-bit → Base32), with dynamic dimensions driven by SNConfig
- Production batches — plan quantity, provision on demand (lazy device creation)
- Device provisioning — generate SN + assign licenses from pool
- Multi-type licensing (MIIT, FCC, CE, ...) with pool management
- Firmware depot per model, CI-friendly (GitHub Actions update via file commit)
- Device log engine (Loki-inspired) — WAL + zstd compression + label-based indexing, hot/cold transparent query
- Full-text search on devices

## Tech Stack

| Component | Technology |
|-----------|-----------|
| Language | Rust |
| HTTP | axum |
| KV Store | redb |
| SQL | SQLite (rusqlite) |
| Search | tantivy |
| Compression | zstd |
| Auth | JWT (jsonwebtoken) |
| Build | Bazel |

## Quick Start

```bash
# Auth service
openerp-auth --data-dir=./config --listen=0.0.0.0:8080

# PMS service
openerp-pms --data-dir=./config --db=./data.redb --listen=0.0.0.0:8081
```

## License

Apache-2.0
