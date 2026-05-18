use std::error::Error;
use std::process::Command;

use crate::models::bonds::BondListItem;

pub fn analyze_bond(bond: &BondListItem) -> Result<String, Box<dyn Error + Send + Sync>> {
	let prompt = build_prompt(bond);

	let output = Command::new("opencode")
		.arg("run")
		.arg(&prompt)
		.output()
		.map_err(|e| {
			Box::new(std::io::Error::other(
				format!("Failed to execute opencode: {}", e),
			)) as Box<dyn std::error::Error + Send + Sync>
		})?;

	if !output.status.success() {
		let stderr =
			String::from_utf8(output.stderr).unwrap_or_else(|_| "Unknown error".to_string());
		eprintln!("opencode command failed: {}", stderr);
		return Err(Box::new(std::io::Error::other(
			format!("opencode command failed: {}", stderr),
		)) as Box<dyn std::error::Error + Send + Sync>);
	}

	let result = String::from_utf8(output.stdout).map_err(|e| {
		eprintln!("Failed to parse opencode output: {}", e);
		Box::new(std::io::Error::other(
			format!("Failed to parse opencode output: {}", e),
		)) as Box<dyn std::error::Error + Send + Sync>
	})?;

	Ok(result.trim().to_string())
}

fn build_prompt(bond: &BondListItem) -> String {
	let name = &bond.name;
	let ticker = &bond.ticker;
	let price = bond
		.price
		.map(|p| p.to_string())
		.unwrap_or_else(|| "N/A".to_string());
	let yield_to_maturity = bond
		.yield_to_maturity
		.map(|y| y.to_string())
		.unwrap_or_else(|| "N/A".to_string());
	let coupon_type = bond.coupon_type.as_deref().unwrap_or("N/A");
	let next_coupon = bond.next_coupon.as_deref().unwrap_or("N/A");
	let maturity = bond.maturity.as_deref().unwrap_or("N/A");
	let nominal = "1000".to_string();
	let accrued = bond
		.accrued_coupon_income
		.map(|v| v.to_string())
		.unwrap_or_else(|| "N/A".to_string());
	let coupon_amount = bond
		.coupon_amount
		.map(|v| v.to_string())
		.unwrap_or_else(|| "N/A".to_string());
	let payments = bond
		.payments_per_year
		.map(|v| v.to_string())
		.unwrap_or_else(|| "N/A".to_string());
	let subordinated = bond.subordinated.as_deref().unwrap_or("N/A");
	let amortization = bond.amortization.as_deref().unwrap_or("N/A");

	format!(
        "Думай шаг за шагом. Не галлюцинируй. Не выдумывай внешние данные (ставки ЦБ, кредитные рейтинги, волатильность). \
        Используй только предоставленные параметры. Если данных недостаточно для точного расчёта, явно укажи допущение и дай диапазон.\n\n\
        Ты — аналитик фиксированного дохода с 20-летним стажем на российском рынке облигаций. \
        Проведи расчёт риск-модели облигации и дай структурированную оценку.\n\n\
        === ИНСТРУКЦИЯ ПО РАСЧЁТУ ===\n\
        1. ПРОЦЕНТНЫЙ РИСК:\n\
           - Рассчитай ориентировочную модифицированную дюрацию (D_mod). \
             Если точные денежные потоки неизвестны, используй приближение: D_mod ≈ (Срок до погашения в годах) × (1 / (1 + YTM/freq)).\n\
           - Оцени изменение цены при сдвиге кривой на +1%: ΔP(%) ≈ -D_mod × 1%.\n\
           - Рассчитай DV01 = (Цена × D_mod × 0.0001) на 1 облигацию.\n\n\
        2. КРЕДИТНЫЙ РИСК:\n\
           - Оцени по параметрам: субординированность, амортизация, частота выплат, срок до погашения.\n\
           - Присвой уровень: Низкий / Умеренный / Высокий + краткое обоснование.\n\n\
        3. СТРУКТУРНЫЕ И ЛИКВИДНОСТНЫЕ РИСКИ:\n\
           - Учти тип купона (плавающий/фиксированный), наличие амортизации, субординацию.\n\
           - Оцени сложность закрытия позиции без проскальзывания.\n\n\
        4. ИТОГОВЫЙ РИСК-СКОРИНГ:\n\
           - Суммируй факторы, выставь риск-профиль: Низкий / Умеренный / Высокий / Спекулятивный.\n\
           - Укажи, какие макро-факторы могут резко изменить оценку.\n\n\
        === ДАННЫЕ ОБЛИГАЦИИ ===\n\
        - Название: {0}\n\
        - Тикер: {1}\n\
        - Цена: {2}₽\n\
        - Доходность к погашению (YTM): {3}%\n\
        - Тип купона: {4}\n\
        - Дата следующего купона: {5}\n\
        - Дата погашения: {6}\n\
        - Номинал: {7}₽\n\
        - Накопленный купонный доход (НКД): {8}₽\n\
        - Величина купона: {9}₽\n\
        - Выплат в год: {10}\n\
        - Субординированность: {11}\n\
        - Амортизация: {12}\n\n\
        === ОТВЕТЬ СТРОГО В ФОРМАТЕ ===\n\
        [ОЦЕНКА]: Покупать / Держать / Продавать\n\
        [ДИСКОНТ/ПРЕМИЯ]: Цена отличается от номинала на X% (укажи формулу расчёта)\n\
        [РИСК-МОДЕЛЬ]:\n\
          • Процентный риск: D_mod = ..., ΔP при +1% = ..., DV01 = ...\n\
          • Кредитный риск: [Уровень]. Обоснование: ...\n\
          • Структурные риски: [Описание влияния амортизации/субординации/типа купона]\n\
          • Итоговый профиль: [Категория]. Ключевые триггеры изменения: ...\n\
        [ОБОСНОВАНИЕ]: 2-3 предложения, почему выбрана такая оценка с учётом риск-модели и текущих параметров.\n\
        [ОГРАНИЧЕНИЯ]: Что не учтено из-за отсутствия данных (например, волатильность ставок, кредитный рейтинг эмитента, ликвидность стакана).",
        name, ticker, price, yield_to_maturity, coupon_type, next_coupon,
        maturity, nominal, accrued, coupon_amount, payments, subordinated, amortization
    )
}
