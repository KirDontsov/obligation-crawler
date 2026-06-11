# Roadmap — V1

**Project:** obligation_crawler
**Mode:** YOLO

## Phases

### Phase 1: Infrastructure Setup

**Goal:** Настроить базовую инфраструктуру для микросервиса.

**Requirements:** REQ-001, REQ-006

**Tasks:**

- Настроить CrawlerConfig для RabbitMQ подключения
- Добавить структуру сообщений RabbitMQ (входящие задачи)
- Реализовать RabbitMQConsumer с retry логикой
- Добавить lifecycle management (run status tracking)

**Verification:**

- [ ] Consumer подключается к RabbitMQ
- [ ] Статусы run записываются в БД

---

### Phase 2: Phase 1 Parser — Quick List

**Goal:** Реализовать быстрый прогон по списку облигаций.

**Requirements:** REQ-002

**Tasks:**

- Извлечь текущую логику парсинга из bonds_crawler.rs
- Создать парсер фазы 1 (только список, без деталей)
- Интегрировать с WebDriver (thirtyfour)
- Сохранять preliminary результаты

**Verification:**

- [ ] Парсит список облигаций с главной страницы
- [ ] Извлекает ticker, name, price
- [ ] Не открывает детальные вкладки

---

### Phase 3: Phase 2 Parser — Detailed

**Goal:** Реализовать детальный парсинг каждой облигации.

**Requirements:** REQ-003

**Tasks:**

- Создать парсер фазы 2 (детальный прогон)
- Открывает каждую облигацию по очереди
- Извлекает все поля: yield, coupon, maturity, volume, etc.
- Обрабатывает недоступные облигации

**Verification:**

- [ ] Открывает детальную страницу каждой облигации
- [ ] Извлекает полные данные согласно схеме БД
- [ ] Обрабатывает ошибки парсинга

---

### Phase 4: PostgreSQL Integration

**Goal:** Интегрировать запись в PostgreSQL.

**Requirements:** REQ-004, REQ-006

**Tasks:**

- Настроить подключение к PostgreSQL (sqlx)
- Реализовать запись в obligation_crawler_runs
- Реализовать запись в obligation_crawler_bonds
- Добавить обработку ошибок БД

**Verification:**

- [ ] Run записывается в БД
- [ ] Bonds записываются в БД
- [ ] Обработка ошибок работает

---

### Phase 5: RabbitMQ Producer — Notifications

**Goal:** Публиковать уведомления о завершении в RabbitMQ.

**Requirements:** REQ-005

**Tasks:**

- Создать RabbitMQProducer (по образцу analytics_publisher)
- Реализовать формат сообщения о завершении
- Интегрировать после успешного парсинга
- Обработать ошибки отправки

**Verification:**

- [ ] Сообщение публикуется при завершении
- [ ] Формат соответствует образцу
- [ ] Обработка ошибок работает

---

### Phase 6: Metrics & Polish

**Goal:** Добавить метрики и финальные улучшения.

**Requirements:** REQ-007

**Tasks:**

- Добавить логирование ключевых событий
- Записать метрики в БД (duration, bond count, errors)
- Провести интеграционное тестирование
- Почистить код от прототипных артефактов

**Verification:**

- [ ] Логи содержат все ключевые события
- [ ] Метрики записываются правильно
- [ ] End-to-end flow работает

### Phase 7: Strip Primary Analysis and Idempotent Bond Refresh

**Goal:** Превратить краулер в чистый scrape+parse+persist и сделать повторные прогоны идемпотентными. Убрать AI-анализ (opencode) из краулера целиком — ответственность за первичный анализ переходит к downstream API. Перестать дублировать облигации между прогонами: на повторном прогоне находить существующие бумаги по тикеру, сравнивать поля и обновлять только изменившиеся (чаще всего — цену). Перейти от append-only run-snapshot модели к текущему состоянию (одна строка на тикер, upsert) + отдельный лёгкий лог истории цен.

**Requirements:** REQ-008, REQ-009

**Depends on:** Phase 4

**Plans:** 3 plans

Plans:
- [ ] 07-01-PLAN.md — Удалить opencode-интеграцию (REQ-009): чистый scrape+parse, analysis всегда NULL
- [ ] 07-02-PLAN.md — Миграция 002 (REQ-008): дедуп + UNIQUE(ticker) + updated_at + price_history + сохранение read-контракта latest_bonds view
- [ ] 07-03-PLAN.md — Репозиторий (REQ-008): ON CONFLICT (ticker) DO UPDATE, запись price_history при изменении цены, счётчики created/updated/unchanged

**Tasks:**
- Удалить opencode-интеграцию: `src/services/opencode_service.rs` и её вызов в `bonds_crawler.rs` (~207), включая skip-analysis логику (~175-216)
- Убрать поле `analysis` из пути записи краулера (колонка остаётся в схеме для downstream API, но краулер её больше не заполняет)
- Миграция: уникальный ключ по `ticker` для upsert; новая таблица `obligation_crawler_price_history`
- Репозиторий: `INSERT ... ON CONFLICT (ticker) DO UPDATE` только по изменившимся полям; запись в price_history при изменении цены/купона
- Сохранить read-контракт для API: семантика "latest per ticker" (вью `obligation_crawler_latest_bonds`) должна продолжать работать
- Логировать число created / updated / unchanged по итогам прогона

**Verification:**
- [ ] Краулер собирается без opencode-кода; повторный прогон не создаёт дубликатов (одна строка на тикер)
- [ ] Изменение цены отражается в bonds и добавляет строку в price_history; неизменные поля не трогаются
- [ ] API по-прежнему читает данные без изменений своего кода

---

## Dependencies

```
Phase 1 (Infrastructure)
    ↓
Phase 2 (Phase 1 Parser) ← Phase 3 (Phase 2 Parser) - can be parallel after Phase 2
    ↓
Phase 4 (PostgreSQL)
    ↓
Phase 5 (RabbitMQ Producer)
    ↓
Phase 6 (Metrics & Polish)
    ↓
Phase 7 (Strip Analysis + Idempotent Refresh)
```

## Notes

- Фазы 2 и 3 можно делать последовательно (3 зависит от результатов 2)
- Phase 4 зависит от Phase 1 (нужна структура run для записи)
- Phase 5 зависит от Phase 4 (нужны записанные данные для уведомления)
- Phase 7 зависит от Phase 4 (меняет схему и путь записи в БД); кросс-репо: read-контракт obligation-api сохраняется без изменений кода API
