# State — obligation_crawler

**Last Updated:** 2026-05-19

## Project Status

- **Initialized:** 2026-05-19 via /gsd-new-project
- **Mode:** YOLO
- **Current Phase:** Not started

## Codebase Context

Mapped via /gsd-map-codebase (2026-05-19):
- STACK.md — Technology stack (Rust, Tokio, thirtyfour, lapin, sqlx)
- ARCHITECTURE.md — Services layer + Repository + Models
- STRUCTURE.md — src/services/, src/models/, src/repository/
- CONVENTIONS.md — No unwrap(), log macros, CrawlerError pattern
- TESTING.md — Project testing structure
- INTEGRATIONS.md — PostgreSQL, RabbitMQ, Selenium
- CONCERNS.md — Tech debt and issues

## Requirements

**V1 (In Progress):**
- REQ-001: RabbitMQ Consumer
- REQ-002: T-Bank Parser Phase 1 (Quick List)
- REQ-003: T-Bank Parser Phase 2 (Detailed)
- REQ-004: PostgreSQL Write
- REQ-005: RabbitMQ Producer (Completion Notification)
- REQ-006: Crawler Run Lifecycle
- REQ-007: Basic Metrics

**V2 (Deferred):**
- Другие парсеры
- Вынос анализа в отдельный микросервис

## Key Decisions

1. **2-фазный парсинг:** Фаза 1 = quick list, Фаза 2 = detailed
2. **Поток:** RabbitMQ → Parse (2 phases) → PostgreSQL → RabbitMQ notification
3. **RabbitMQ формат:** По образцу analytics_publisher.rs
4. **PostgreSQL схема:** Использовать существующую из migrations/

## Integrations

- **PostgreSQL:** existing schema (`001_create_crawler_schema.sql`)
- **RabbitMQ:** avito_exchange (по образцу)
- **T-Bank:** Selenium via thirtyfour

## Open Questions

- Конкретный формат входящих сообщений RabbitMQ (пока не продуман)
- Название очередей и routing keys (по образцу существующих сервисов)

## Recent Changes

- 2026-05-19: Created project via /gsd-map-codebase + /gsd-new-project
- 2026-05-19: Generated codebase map (.planning/codebase/)
- 2026-05-19: Fixed all clippy warnings (61 issues):
  - Removed unused imports (8 instances)
  - Added #[allow(dead_code)] to unused structs/functions/fields (26 items)
  - Added #[allow(deprecated)] for chrono::from_timestamp_opt
  - Added #[allow(clippy::too_many_arguments)] for config::new()
  - Added #[allow(clippy::enum_variant_names)] for CrawlerError
  - Fixed needless_borrows_for_generic_args (2 instances)
  - Fixed collapsible_if in bonds_crawler.rs
  - Fixed option_as_ref_deref (14 instances → as_deref())
  - Fixed useless_format (format!("...") → "...".to_string())
  - Fixed single_match (match → if let)
  - Fixed io_other_error (3 instances → Error::other())
  - Fixed redundant_closure (4 instances)
  - Fixed println_empty_string (empty string literal)
  - Added #[allow(clippy::result_large_err)] in main.rs
  - Updated import paths to use full module paths after removing re-exports