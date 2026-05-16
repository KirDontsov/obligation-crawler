use std::process::Command;
use std::error::Error;

use crate::models::bonds::BondListItem;

pub fn analyze_bond(bond: &BondListItem) -> Result<String, Box<dyn Error + Send + Sync>> {
    let prompt = build_prompt(bond);
    
    let output = Command::new("opencode")
        .arg("run")
        .arg(&prompt)
        .output()
        .map_err(|e| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to execute opencode: {}", e),
            )) as Box<dyn std::error::Error + Send + Sync>
        })?;
    
    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr).unwrap_or_else(|_| "Unknown error".to_string());
        eprintln!("opencode command failed: {}", stderr);
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("opencode command failed: {}", stderr),
        )) as Box<dyn std::error::Error + Send + Sync>);
    }
    
    let result = String::from_utf8(output.stdout).map_err(|e| {
        eprintln!("Failed to parse opencode output: {}", e);
        Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to parse opencode output: {}", e),
        )) as Box<dyn std::error::Error + Send + Sync>
    })?;
    
    Ok(result.trim().to_string())
}

fn build_prompt(bond: &BondListItem) -> String {
    let name = &bond.name;
    let ticker = &bond.ticker;
    let price = bond.price.map(|p| p.to_string()).unwrap_or_else(|| "N/A".to_string());
    let yield_to_maturity = bond.yield_to_maturity.map(|y| y.to_string()).unwrap_or_else(|| "N/A".to_string());
    let coupon_type = bond.coupon_type.as_deref().unwrap_or("N/A");
    let next_coupon = bond.next_coupon.as_deref().unwrap_or("N/A");
    let maturity = bond.maturity.as_deref().unwrap_or("N/A");
    let nominal = "1000".to_string();
    let accrued = bond.accrued_coupon_income.map(|v| v.to_string()).unwrap_or_else(|| "N/A".to_string());
    let coupon_amount = bond.coupon_amount.map(|v| v.to_string()).unwrap_or_else(|| "N/A".to_string());
    let payments = bond.payments_per_year.map(|v| v.to_string()).unwrap_or_else(|| "N/A".to_string());
    let subordinated = bond.subordinated.as_deref().unwrap_or("N/A");
    let amortization = bond.amortization.as_deref().unwrap_or("N/A");
    
    format!(
        "Думай шаг за шагом. Не галлюцинируй. Не выдумывай ложные данные.\n\n\
Ты — опытный инвестор с 20-летним стажем на российском рынке облигаций. \
Проанализируй облигацию и дай структурированную оценку в следующем формате:\n\n\
ОЦЕНКА: [Покупать/Держать/Продавать]\nДИСКОНТ/ПРЕМИЯ: [На сколько % цена отличается от номинала]\nРИСКИ: [1-2 главных риска]\nДОХОДНОСТЬ: [Оценка доходности с пояснением]\n\n\
Данные облигации:\n\
- Название: {}\n\
- Тикер: {}\n\
- Цена: {}₽\n\
- Доходность к погашению: {}%\n\
- Тип купона: {}\n\
- Дата следующего купона: {}\n\
- Дата погашения: {}\n\
- Номинал: {}₽\n\
- Накопленный купонный доход: {}₽\n\
- Величина купона: {}₽\n\
- Выплат в год: {}\n\
- Субординированность: {}\n\
- Амортизация: {}",
        name, ticker, price, yield_to_maturity, coupon_type, next_coupon, 
        maturity, nominal, accrued, coupon_amount, payments, subordinated, amortization
    )
}