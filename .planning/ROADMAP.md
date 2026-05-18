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
```

## Notes

- Фазы 2 и 3 можно делать последовательно (3 зависит от результатов 2)
- Phase 4 зависит от Phase 1 (нужна структура run для записи)
- Phase 5 зависит от Phase 4 (нужны записанные данные для уведомления)