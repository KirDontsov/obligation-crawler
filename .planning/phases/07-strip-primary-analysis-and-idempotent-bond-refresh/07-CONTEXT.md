# Phase 7: Strip Primary Analysis and Idempotent Bond Refresh — Context

**Gathered:** 2026-06-11
**Status:** Ready for planning
**Source:** Direct user decisions (cross-repo rework: crawler + obligation-api)

<domain>
## Phase Boundary

Этот фаза охватывает ТОЛЬКО краулер (`obligation-crawler`). Два изменения:

1. **Удалить первичный AI-анализ из краулера.** Краулер становится чистым
   scrape + parse + persist. Интеграция с opencode уходит целиком.
2. **Сделать повторные прогоны идемпотентными.** Вместо append-only вставки
   (которая дублирует все облигации на каждом прогоне) — обновление существующих
   бумаг по тикеру с диффом полей, плюс отдельный лог истории цен.

**Контекст более крупной работы (НЕ в этой фазе):** Первичный анализ
переезжает в `obligation-api` — там уже есть метод анализа, который станет
ОСНОВНЫМ инструментом первичного анализа (с WebFetch-обогащением и переработанным
промптом). Это отдельная фаза в репозитории API. **Не планировать здесь изменения
API.**

Это реализует уже задеклаированный в проекте deferred-пункт:
REQUIREMENTS.md → "Вынос анализа в отдельный микросервис".
</domain>

<decisions>
## Implementation Decisions (LOCKED)

### Удаление анализа
- Удалить `src/services/opencode_service.rs` целиком.
- Удалить вызов `analyze_bond(...)` в `src/services/bonds_crawler.rs` (~строка 207)
  и всю skip-analysis логику (~175–216: пропуск по сроку < 1 года и цене > номинал+5).
- Колонка `analysis` ОСТАЁТСЯ в схеме БД (её будет заполнять downstream API).
  Краулер просто перестаёт её писать → значение остаётся NULL.
- Убрать `opencode` из любых зависимостей/проверок PATH в коде краулера.

### Идемпотентный refresh
- **Идентификатор бумаги между прогонами = `ticker`.** Краулер не собирает ISIN,
  поэтому ticker — единственный стабильный ключ. Добавить UNIQUE constraint на ticker.
- **Модель данных (решение пользователя): current-state + price-history log.**
  - `obligation_crawler_bonds` становится таблицей текущего состояния: ОДНА строка
    на тикер, обновляется in place (`INSERT ... ON CONFLICT (ticker) DO UPDATE`).
  - Новая таблица `obligation_crawler_price_history`: строка добавляется ТОЛЬКО когда
    меняется цена (и/или купон). Хранит как минимум ticker, price, (опц. coupon_amount),
    timestamp, run_id. Это сохраняет тренд и решает проблему staleness, не дублируя
    весь набор данных каждый прогон.
- **Дифф полей:** на повторном прогоне сравнить распарсенные значения с сохранёнными;
  обновлять только изменившиеся поля. Большинство полей (name, maturity, coupon_type,
  флаги) меняются редко — обновляется в основном `price`, реже `coupon_amount`,
  `accrued_coupon_income`, `yield_to_maturity`.
- **`created_at` vs `updated_at`:** `created_at` фиксируется при первой вставке и НЕ
  меняется на update; ввести `updated_at` (или эквивалент), отражающий последнее касание.
  (Планировщик уточняет точные имена, сверяясь с тем, что читает API.)

### Совместимость с downstream API (КРИТИЧНО — read-контракт нельзя ломать)
- API (Diesel) читает данные краулера и ожидает семантику "latest per ticker"
  через вью `obligation_crawler_latest_bonds`. Модель API `LatestBond` включает поля
  `run_id` и `created_at` среди прочих — все читаемые API колонки должны сохраниться.
- После перехода на одну строку на тикер вью "latest" становится тривиальным
  (по сути = сама таблица), но он ДОЛЖЕН продолжать возвращать те же колонки, чтобы
  API не требовал изменений кода.
- Перед планированием свериться с фактическими SELECT'ами/моделью в API
  (`obligation-api/src/models/bonds.rs`), чтобы гарантировать совместимость колонок.

### Lifecycle / runs
- Таблица `obligation_crawler_runs` и lifecycle (running→completed|failed, метрики)
  СОХРАНЯЮТСЯ. Меняется только то, КАК bonds связаны с прогонами: bond-строка теперь
  отражает последний прогон, который её коснулся (`run_id` = последний), а историю
  "когда что менялось" несёт price_history.
- По итогам прогона логировать счётчики: created / updated / unchanged.
</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Текущая схема и запись (то, что меняем)
- `migrations/001_create_crawler_schema.sql` — текущая схема: `obligation_crawler_bonds`
  (append-only, FK `run_id`), `obligation_crawler_runs`, вью `obligation_crawler_latest_bonds`.
- `src/repository/bonds_repository.rs` — путь записи (~100–142): сейчас plain `INSERT`
  без ON CONFLICT → источник дублей. `save_bond`, структура `BondRecord`.
- `src/models/bonds.rs` — `BondListItem` (in-memory модель, ~21–39).

### Анализ (то, что удаляем)
- `src/services/opencode_service.rs` — вся opencode-интеграция и промпт (удалить).
- `src/services/bonds_crawler.rs` — вызов анализа (~207) и skip-логика (~175–216).

### Известные баги/долги (учесть, но не обязательно чинить все здесь)
- `.planning/codebase/CONCERNS.md` — в т.ч.: дублирование между прогонами (этот фикс),
  захардкоженный nominal=1000, инвертированная логика `final_maturity` (~149),
  блокирующий subprocess в async-контексте (уходит вместе с анализом),
  `println!` вместо `log`, русские комментарии (нарушение English-only в коде).

### Downstream contract (не менять, только соблюдать)
- `obligation-api/src/models/bonds.rs` — `LatestBond` и SELECT'ы, читающие краулерные
  таблицы/вью. Сверить набор колонок ПЕРЕД изменением схемы.

### Конвенции краулера
- `.planning/codebase/CONVENTIONS.md` — no `unwrap()`, `log` макросы, паттерн `CrawlerError`.
- Стек: Rust + Tokio + thirtyfour + **sqlx** (raw SQL) + lapin. (Краулер на sqlx, не Diesel.)
</canonical_refs>

<specifics>
## Specific Ideas

- Upsert на sqlx: `INSERT INTO obligation_crawler_bonds (...) VALUES (...) ON CONFLICT (ticker)
  DO UPDATE SET price = EXCLUDED.price, ... WHERE <что-то изменилось>`; либо app-level дифф
  с последующим UPDATE только изменившихся колонок. Планировщик выбирает подход и обосновывает.
- price_history: писать строку при изменении цены. Решить, писать ли также при изменении
  купона; минимально — цена.
- Миграция должна быть аккуратной к существующим дублирующимся строкам (если в БД уже есть
  дубли от старых прогонов) — стратегия дедупа перед добавлением UNIQUE(ticker), либо чистый
  пересоздающий путь, если данные одноразовые. Планировщик уточняет, есть ли продовые данные.
- Удаление анализа также убирает блокирующий subprocess-вызов в async-контексте — попутный плюс.
</specifics>

<deferred>
## Deferred Ideas (НЕ в этой фазе)

- Вся работа на стороне API: перенос/доработка первичного анализа, WebFetch-обогащение
  (рейтинги, ключевая ставка, новости), переработка промпта — отдельная фаза в `obligation-api`.
- Очистка прочих долгов краулера (nominal=1000, `final_maturity`, миграция `println!`→`log`)
  — точечно, только если попадает в путь изменяемого кода; полноценная зачистка вне scope.
- Изменение формата/контента RabbitMQ-уведомлений.
</deferred>

---

*Phase: 07-strip-primary-analysis-and-idempotent-bond-refresh*
*Context gathered: 2026-06-11 (direct user decisions)*
