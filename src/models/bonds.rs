use chrono::{DateTime, Utc};
use csv::Writer;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bond {
    pub name: String,
    pub ticker: String,
    pub price: f64,
    pub yield_to_maturity: f64,
    pub coupon_rate: f64,
    pub next_coupon_date: Option<String>,
    pub maturity_date: String,
    pub volume: i64,
    pub change_percent: f64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondListItem {
    pub ticker: String,
    pub name: String,
    pub price: Option<f64>,
    pub yield_to_maturity: Option<f64>,
    pub coupon_type: Option<String>,
    pub next_coupon: Option<String>,
    pub maturity: Option<String>,
    pub volume: Option<i64>,
    pub accrued_coupon_income: Option<f64>,
    pub coupon_amount: Option<f64>,
    pub payments_per_year: Option<i32>,
    pub subordinated: Option<String>,
    pub amortization: Option<String>,
    pub for_qualified_investors: Option<String>,
    pub change_today: Option<f64>,
    pub analysis: Option<String>,
}

impl BondListItem {
    pub fn create_csv_file(filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all("./output")?;
        
        let mut wtr = Writer::from_path(filename)?;
        
        wtr.write_record(&[
            "Название",
            "Тикер",
            "Цена",
            "Доходность к погашению",
            "Тип купона",
            "Дата выплаты купона",
            "Дата погашения",
            "Номинал",
            "Накопленный купонный доход",
            "Величина купона",
            "Количество выплат в год",
            "Субординированность",
            "Амортизация",
            "Для квалифицированных инвесторов",
            "Анализ",
        ])?;
        
        wtr.flush()?;
        println!("CSV файл создан: {}", filename);
        Ok(())
    }

    pub fn append_to_csv(bond: &BondListItem, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Открываем файл в режиме добавления
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(filename)?;
        
        let mut wtr = Writer::from_writer(file);
        
        wtr.write_record(&[
            &bond.name,
            &bond.ticker,
            &bond.price.map(|p| p.to_string()).unwrap_or_default(),
            &bond.yield_to_maturity.map(|y| y.to_string()).unwrap_or_default(),
            &bond.coupon_type.clone().unwrap_or_default(),
            &bond.next_coupon.clone().unwrap_or_default(),
            &bond.maturity.clone().unwrap_or_default(),
            &bond.volume.map(|v| v.to_string()).unwrap_or_default(),
            &bond.accrued_coupon_income.map(|v| v.to_string()).unwrap_or_default(),
            &bond.coupon_amount.map(|v| v.to_string()).unwrap_or_default(),
            &bond.payments_per_year.map(|v| v.to_string()).unwrap_or_default(),
            &bond.subordinated.clone().unwrap_or_default(),
            &bond.amortization.clone().unwrap_or_default(),
            &bond.for_qualified_investors.clone().unwrap_or_default(),
            &bond.analysis.clone().unwrap_or_default(),
        ])?;
        
        wtr.flush()?;
        Ok(())
    }
}