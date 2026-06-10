# Project: obligation_crawler

**Created:** 2026-05-19

## Vision

Микросервис-краулер для парсинга облигаций Т-Банка с интеграцией в существующую инфраструктуру (PostgreSQL + RabbitMQ).

## Current State

Существующий прототип Rust-краулера:
- Парсит облигации с T-Bank через Selenium (thirtyfour)
- AI анализ через opencode CLI
- CSV вывод
- PostgreSQL + RabbitMQ опционально

## Target State (V1)

Полноценный микросервис:
- 2-фазный парсинг облигаций (quick list → detailed)
- Слушает RabbitMQ очередь для запуска
- Записывает данные в PostgreSQL
- Публикует уведомления в RabbitMQ о завершении
- Без API/аутентификации (чистый микросервис)

## Scope

### V1 (Immediate)
- [x] T-Bank bonds парсер (2 фазы: быстрый прогон + детальный)
- [x] PostgreSQL запись (использовать существующую схему)
- [x] RabbitMQ consumer — слушает задачи
- [x] RabbitMQ producer — уведомления о завершении
- [x] Метрики/мониторинг (базовый)

### V2 (Deferred)
- [ ] Другие парсеры (источники)
- [ ] Вынос анализа в отдельный микросервис

## Integration Points

**PostgreSQL:** Существующая схема (`migrations/001_create_crawler_schema.sql`)
- `obligation_crawler_runs` — сессии краулинга
- `obligation_crawler_bonds` — облигации

**RabbitMQ:** По образцу существующего сервиса (`a_back_deploy`)
- Exchange: `avito_exchange`
- Входящая очередь: задачи парсинга
- Исходящий routing key: уведомления о завершении

## Key Decisions

1. **2-фазный парсинг:**
   - Фаза 1: Быстрый прогон по списку (ID, название, цена — без открытия вкладок)
   - Фаза 2: Детальный прогон (открыть каждую облигацию, полные данные)

2. **Поток данных:**
   - Consumer получает задачу из RabbitMQ
   - Парсинг облигаций (2 фазы)
   - Запись в PostgreSQL
   - Публикация уведомления в RabbitMQ

3. **Сообщения RabbitMQ:** По образцу `analytics_publisher.rs`

## Constraints

- Без аутентификации (микросервис)
- Rust 2021 edition
- Tokio async runtime
- thirtyfour для Selenium
- sqlx для PostgreSQL
- lapin для RabbitMQ