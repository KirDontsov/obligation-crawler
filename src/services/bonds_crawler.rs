use crate::config::CrawlerConfig;
use crate::error::{CrawlerError, Result};
use crate::models::bonds::BondListItem;
use crate::repository::BondsRepository;
use crate::services::opencode_service::analyze_bond;
use chrono::Utc;
use log::{info, warn};
use thirtyfour::{ChromiumLikeCapabilities, DesiredCapabilities, WebDriver, WebElement};
use tokio::time::{sleep, Duration};
use uuid::Uuid;

async fn parse_bond_row_inner(
	driver: &WebDriver,
	row: &WebElement,
	csv_filename: &Option<String>,
) -> Result<Option<BondListItem>> {
	// Проверяем, что элемент все еще валиден
	let cells = match row.find_all(thirtyfour::By::Css("td")).await {
		Ok(c) => c,
		Err(e) => {
			println!("[DEBUG] Не удалось найти ячейки в строке: {:?}", e);
			return Ok(None);
		}
	};

	let mut bond_name = String::new();
	let mut bond_ticker = String::new();
	let mut maturity_date = String::new();
	let mut yield_to_maturity: Option<f64> = None;
	let mut price: Option<f64> = None;

	if cells.len() >= 4 {
		// cell 0 - Название облигации и тикер (ищем внутри специфические элементы)
		if let Ok(name_div) = cells[0]
			.find(thirtyfour::By::Css(".SecurityRow__showName_inlal"))
			.await
		{
			bond_name = name_div.text().await.unwrap_or_default();
		}
		if let Ok(ticker_div) = cells[0]
			.find(thirtyfour::By::Css(".SecurityRow__ticker_KMm7A"))
			.await
		{
			bond_ticker = ticker_div.text().await.unwrap_or_default();
		}

		// cell 1 - Дата погашения
		if let Ok(date_div) = cells[1]
			.find(thirtyfour::By::Css(".BondsTable__dateToClient_LjMTe"))
			.await
		{
			maturity_date = date_div.text().await.unwrap_or_default();
		} else {
			// Пробуем просто получить весь текст из ячейки
			maturity_date = cells[1].text().await.unwrap_or_default();
		}

		// cell 2 - Доходность - ищем вложенный элемент с классом Money
		let yield_text = if let Ok(money_elem) = cells[2]
			.find(thirtyfour::By::Css(".Money-module__money_UZBbh"))
			.await
		{
			money_elem.text().await.unwrap_or_default()
		} else {
			cells[2].text().await.unwrap_or_default()
		};
		let yield_clean = yield_text
			.replace("\n", "")
			.replace("\r", "")
			.replace("\t", "")
			.replace(" ", "")
			.replace("\u{A0}", "")
			.replace("%", "")
			.replace(",", ".")
			.trim()
			.to_string();
		yield_to_maturity = yield_clean.parse().ok();

		// cell 3 - Цена - ищем вложенный элемент с классом Money
		let price_text = if let Ok(money_elem) = cells[3]
			.find(thirtyfour::By::Css(".Money-module__money_UZBbh"))
			.await
		{
			money_elem.text().await.unwrap_or_default()
		} else {
			cells[3].text().await.unwrap_or_default()
		};
		let price_clean = price_text
			.replace("\n", "")
			.replace("\r", "")
			.replace("\t", "")
			.replace(" ", "")
			.replace("\u{A0}", "")
			.replace("₽", "")
			.replace(",", ".")
			.trim()
			.to_string();
		price = price_clean.parse().ok();

		println!(
			"[DEBUG] Строка: {} | {} | {} | Доходность: {}% | Цена: {}₽",
			bond_name,
			bond_ticker,
			maturity_date,
			yield_to_maturity
				.map(|y| y.to_string())
				.unwrap_or_else(|| "N/A".to_string()),
			price
				.map(|p| p.to_string())
				.unwrap_or_else(|| "N/A".to_string())
		);
	}

	// Теперь находим ссылку и переходим по ней
	let link: WebElement = row
		.find(thirtyfour::By::Css("a[data-qa-file=\"TableLinkCell\"]"))
		.await
		.map_err(|e| CrawlerError::SeleniumError(format!("Link not found: {}", e)))?;

	let href: String = link
		.attr("href")
		.await?
		.ok_or_else(|| CrawlerError::SeleniumError("No href found".to_string()))?;

	let main_window = driver.window().await?;

	driver
		.execute(&format!("window.open('{}', '_blank')", href), Vec::new())
		.await?;

	sleep(Duration::from_secs(1)).await;

	let windows = driver.windows().await?;

	if windows.len() > 1 {
		driver.switch_to_window(windows[1].clone()).await?;

		sleep(Duration::from_secs(2)).await;

		let details = collect_bond_details_inner(driver).await?;

		driver.close_window().await?;

		driver.switch_to_window(main_window).await?;
		sleep(Duration::from_millis(500)).await;

		// Объединяем данные - с главной страницы: price; с детальной: все остальные
		let final_yield = yield_to_maturity.or(details.yield_to_maturity);
		let final_maturity = Some(maturity_date.clone()).or(details.maturity);

		let mut bond_result = BondListItem {
			ticker: bond_ticker.clone(),
			name: format!(
				"{} | {} | {}%",
				bond_name,
				maturity_date,
				final_yield.unwrap_or(0.0)
			),
			price,
			yield_to_maturity: final_yield,
			coupon_type: details.coupon_type,
			next_coupon: details.next_coupon,
			maturity: final_maturity,
			volume: details.nominal.map(|n| n as i64),
			accrued_coupon_income: details.accrued_coupon_income,
			coupon_amount: details.coupon_amount,
			payments_per_year: details.payments_per_year,
			subordinated: details.subordinated,
			amortization: details.amortization,
			for_qualified_investors: details.for_qualified_investors,
			change_today: None,
			analysis: None,
		};

		// Проверяем срок погашения и цену - пропускаем анализ если:
		// 1. Срок погашения меньше 1 года
		// 2. Цена выше номинала более чем на 5 рублей
		let skip_analysis = bond_result
			.maturity
			.as_ref()
			.map(|maturity_str| {
				if let Ok(date) = chrono::NaiveDate::parse_from_str(maturity_str, "%d.%m.%Y") {
					let now = chrono::Utc::now().date_naive();
					let one_year_later = now + chrono::Duration::days(365);
					date < one_year_later
				} else {
					false
				}
			})
			.unwrap_or(false)
			|| bond_result
				.price
				.map(|price| price - 1000.0 > 5.0)
				.unwrap_or(false);

		// Анализируем облигацию через OpenCode (если срок > 1 года)
		if skip_analysis {
			println!(
				"[DEBUG] Пропускаем анализ: срок погашения меньше 1 года - {}",
				bond_result.ticker
			);
		} else {
			println!(
				"[DEBUG] Анализируем облигацию через OpenCode: {}",
				bond_result.ticker
			);
			match analyze_bond(&bond_result) {
				Ok(analysis) => {
					println!("[DEBUG] Анализ получен: {} chars", analysis.len());
					bond_result.analysis = Some(analysis);
				}
				Err(e) => {
					println!("[DEBUG] Ошибка анализа: {}", e);
				}
			}
		}

		// Записываем в CSV сразу после создания
		if let Some(filename) = csv_filename {
			if let Err(e) = BondListItem::append_to_csv(&bond_result, filename) {
				println!("[DEBUG] Ошибка записи в CSV: {}", e);
			}
		}

		Ok(Some(bond_result))
	} else {
		Ok(None)
	}
}

#[derive(Default, Debug)]
struct BondDetails {
	yield_to_maturity: Option<f64>,
	maturity: Option<String>,
	next_coupon: Option<String>,
	coupon_type: Option<String>,
	accrued_coupon_income: Option<f64>,
	coupon_amount: Option<f64>,
	nominal: Option<f64>,
	payments_per_year: Option<i32>,
	subordinated: Option<String>,
	amortization: Option<String>,
	for_qualified_investors: Option<String>,
}

async fn collect_bond_details_inner(driver: &WebDriver) -> Result<BondDetails> {
	sleep(Duration::from_secs(1)).await;

	let tables = driver
		.find_all(thirtyfour::By::Css("table[data-qa-file=\"Table\"]"))
		.await?;

	let mut details = BondDetails::default();

	// Обходим ВСЕ таблицы и ищем каждое поле по label
	for table in tables.iter() {
		let rows = table
			.find_all(thirtyfour::By::Css(
				"tr[data-qa-type=\"uikit/table.tableRow\"]",
			))
			.await?;

		for row in rows {
			let cells = row.find_all(thirtyfour::By::Css("td")).await?;

			if cells.len() >= 2 {
				let label = cells[0].text().await?;
				let value = cells[1].text().await?;

				let label_clean = label
					.trim()
					.replace("\n", "")
					.replace(" ", "")
					.replace("\u{A0}", "")
					.trim()
					.to_string();
				let value_clean = value
					.replace("\n", "")
					.replace(" ", "")
					.replace("\u{A0}", "")
					.trim()
					.to_string();

				let label_str = label_clean.as_str();

				if label_str.contains("Накопленныйкупонныйдоход") {
					if details.accrued_coupon_income.is_none() {
						let val = value_clean
							.replace("₽", "")
							.replace(",", ".")
							.parse::<f64>()
							.ok();
						details.accrued_coupon_income = val;
					}
				} else if label_str.contains("Величинакупона") {
					if details.coupon_amount.is_none() {
						let val = value_clean
							.replace("₽", "")
							.replace(",", ".")
							.parse::<f64>()
							.ok();
						details.coupon_amount = val;
					}
				} else if label_str.contains("Количествовыплатвгод") {
					if details.payments_per_year.is_none() {
						details.payments_per_year = value_clean.parse::<i32>().ok();
					}
				} else if label_str.contains("Дляквалифицированныхинвесторов")
				{
					if details.for_qualified_investors.is_none() {
						details.for_qualified_investors = Some(value.trim().to_string());
					}
				} else if label_str.contains("Доходностькпогашению") {
					if details.yield_to_maturity.is_none() {
						let val = value_clean
							.replace("%", "")
							.replace(",", ".")
							.parse::<f64>()
							.ok();
						details.yield_to_maturity = val;
					}
				} else if label_str.contains("Датапогашенияоблигации") {
					if details.maturity.is_none() {
						details.maturity = Some(value.trim().to_string());
					}
				} else if label_str.contains("Датавыплатыкупона") {
					if details.next_coupon.is_none() {
						details.next_coupon = Some(value.trim().to_string());
					}
				} else if label_str == "Купон" {
					if details.coupon_type.is_none() {
						details.coupon_type = Some(value.trim().to_string());
					}
				} else if label_str.contains("Номинал") {
					if details.nominal.is_none() {
						let val = value_clean
							.replace("₽", "")
							.replace(",", ".")
							.parse::<f64>()
							.ok();
						details.nominal = val;
					}
				} else if label_str.contains("Субординированность") {
					if details.subordinated.is_none() {
						details.subordinated = Some(value.trim().to_string());
					}
				} else if label_str.contains("Амортизация") && details.amortization.is_none() {
						details.amortization = Some(value.trim().to_string());
					}
			}
		}
	}

	println!("[DEBUG] Детали облигации:");
	println!(
		"[DEBUG]   Доходность к погашению: {}%",
		details
			.yield_to_maturity
			.map(|v| v.to_string())
			.unwrap_or_else(|| "N/A".to_string())
	);
	println!(
		"[DEBUG]   Дата погашения облигации: {}",
		details.maturity.as_deref().unwrap_or("N/A")
	);
	println!(
		"[DEBUG]   Дата выплаты купона: {}",
		details.next_coupon.as_deref().unwrap_or("N/A")
	);
	println!(
		"[DEBUG]   Тип купона: {}",
		details.coupon_type.as_deref().unwrap_or("N/A")
	);
	println!(
		"[DEBUG]   Накопленный купонный доход: {}₽",
		details
			.accrued_coupon_income
			.map(|v| v.to_string())
			.unwrap_or_else(|| "N/A".to_string())
	);
	println!(
		"[DEBUG]   Величина купона: {}₽",
		details
			.coupon_amount
			.map(|v| v.to_string())
			.unwrap_or_else(|| "N/A".to_string())
	);
	println!(
		"[DEBUG]   Номинал: {}₽",
		details
			.nominal
			.map(|v| v.to_string())
			.unwrap_or_else(|| "N/A".to_string())
	);
	println!(
		"[DEBUG]   Количество выплат в год: {}",
		details
			.payments_per_year
			.map(|v| v.to_string())
			.unwrap_or_else(|| "N/A".to_string())
	);
	println!(
		"[DEBUG]   Субординированность: {}",
		details.subordinated.as_deref().unwrap_or("N/A")
	);
	println!(
		"[DEBUG]   Амортизация: {}",
		details.amortization.as_deref().unwrap_or("N/A")
	);
	println!(
		"[DEBUG]   Для квалифицированных инвесторов: {}",
		details.for_qualified_investors.as_deref().unwrap_or("N/A")
	);

	Ok(details)
}

pub struct BondsCrawler {
	config: CrawlerConfig,
	driver: Option<WebDriver>,
	csv_filename: Option<String>,
	db_pool: Option<sqlx::PgPool>,
	run_id: Option<Uuid>,
}

impl BondsCrawler {
	pub fn new(config: CrawlerConfig, db_pool: Option<sqlx::PgPool>) -> Self {
		let timestamp = Utc::now().format("%d-%m-%Y_%H-%M-%S").to_string();
		let csv_filename = format!("./output/bonds_{}.csv", timestamp);

		// Создаем файл с заголовками
		if let Err(e) = BondListItem::create_csv_file(&csv_filename) {
			println!("[DEBUG] Не удалось создать CSV файл: {}", e);
		} else {
			println!("[DEBUG] CSV файл создан: {}", csv_filename);
		}

		Self {
			config,
			driver: None,
			csv_filename: Some(csv_filename),
			db_pool,
			run_id: None,
		}
	}

	pub async fn initialize(&mut self) -> Result<()> {
		println!("[DEBUG] Initializing Chrome WebDriver...");

		let mut caps = DesiredCapabilities::chrome();

		if self.config.headless_chrome {
			caps.set_headless()?;
		}

		caps.add_arg("--no-sandbox")?;
		caps.add_arg("--disable-dev-shm-usage")?;
		caps.add_arg("--disable-blink-features=AutomationControlled")?;
		caps.add_arg("--disable-extensions")?;
		caps.add_arg("--disable-popup-blocking")?;
		caps.add_arg("--start-maximized")?;
		caps.add_arg("--disable-infobars")?;
		caps.add_arg("--disable-notifications")?;
		caps.add_arg("--disable-geolocation")?;
		caps.add_arg("--lang=en-US")?;

		let webdriver_url = "http://localhost:9515".to_string();
		self.driver = Some(WebDriver::new(&webdriver_url, caps).await?);

		Ok(())
	}

	pub async fn wait_for_login(&mut self) -> Result<()> {
		println!("Please login in the browser...");

		sleep(Duration::from_secs(self.config.wait_after_login_seconds)).await;
		Ok(())
	}

	pub async fn navigate_to_bonds(&mut self) -> Result<()> {
		let driver = self
			.driver
			.as_mut()
			.ok_or_else(|| CrawlerError::SeleniumError("WebDriver not initialized".to_string()))?;

		driver.goto(&self.config.tbank_url).await?;

		sleep(Duration::from_secs(3)).await;
		Ok(())
	}

	pub async fn check_page_available(&mut self) -> Result<bool> {
		let driver = self
			.driver
			.as_mut()
			.ok_or_else(|| CrawlerError::SeleniumError("WebDriver not initialized".to_string()))?;

		match driver
			.find(thirtyfour::By::Css(
				"tbody[data-qa-type=\"uikit/dataTable.tableBody\"]",
			))
			.await
		{
			Ok(_) => Ok(true),
			Err(_) => Ok(false),
		}
	}

	pub async fn collect_bonds(&mut self) -> Result<Vec<BondListItem>> {
		let driver = self
			.driver
			.as_mut()
			.ok_or_else(|| CrawlerError::SeleniumError("WebDriver not initialized".to_string()))?;

		let csv_filename = self.csv_filename.clone();
		let db_pool = self.db_pool.clone();
		let run_id = self.run_id;
		let mut all_bonds = Vec::new();
		let mut page_num = 1;
		let max_pages = 50;

		loop {
			if page_num > max_pages {
				break;
			}

			let table_body = driver
				.find(thirtyfour::By::Css(
					"tbody[data-qa-type=\"uikit/dataTable.tableBody\"]",
				))
				.await
				.map_err(|e| CrawlerError::SeleniumError(format!("Table not found: {}", e)))?;

			let rows = table_body
				.find_all(thirtyfour::By::Css(
					"tr[data-qa-type=\"uikit/dataTable.tableLinkRow\"]",
				))
				.await
				.map_err(|e| CrawlerError::SeleniumError(format!("Rows not found: {}", e)))?;

			println!(
				"[DEBUG] На странице {} найдено {} строк",
				page_num,
				rows.len()
			);

			// Используем индексы, чтобы находить строки заново для каждой итерации
			for idx in 0..rows.len() {
				println!("[DEBUG] Парсим строку {} из {}", idx + 1, rows.len());

				// Находим строку заново по индексу
				let row_selector = format!(
					"tr[data-qa-type=\"uikit/dataTable.tableLinkRow\"]:nth-of-type({})",
					idx + 1
				);
				let row = match driver.find(thirtyfour::By::Css(&row_selector)).await {
					Ok(r) => r,
					Err(_) => {
						// Пробуем найти через nth
						let all_rows = table_body
							.find_all(thirtyfour::By::Css(
								"tr[data-qa-type=\"uikit/dataTable.tableLinkRow\"]",
							))
							.await?;
						if idx < all_rows.len() {
							all_rows[idx].clone()
						} else {
							continue;
						}
					}
				};

				match parse_bond_row_inner(driver, &row, &csv_filename).await {
					Ok(Some(bond)) => {
						// Save to database immediately (same pattern as CSV append)
						if let (Some(ref pool), Some(rid)) = (&db_pool, run_id) {
							if let Err(e) = BondsRepository::save_bond(pool, rid, &bond).await {
								warn!("Failed to save bond {} to DB: {}", bond.ticker, e);
							}
						}
						all_bonds.push(bond);
					}
					Ok(None) => {
						println!("[DEBUG] Строка {} вернула None", idx + 1);
					}
					Err(e) => {
						println!("[DEBUG] Ошибка при парсинге строки {}: {:?}", idx + 1, e);
					}
				}
			}

			println!(
				"[DEBUG] Страница {} обработана, всего собрано: {}",
				page_num,
				all_bonds.len()
			);

			match try_click_show_more(driver).await {
				Ok(true) => {
					page_num += 1;
					sleep(Duration::from_secs(1)).await;
					continue;
				}
				Ok(false) => {
					break;
				}
				Err(_) => {
					break;
				}
			}
		}

		Ok(all_bonds)
	}

	pub async fn run_crawl_loop(
		&mut self,
		duration_minutes: Option<u64>,
	) -> Result<Vec<BondListItem>> {
		// Create a DB run record before starting the crawl
		if let Some(ref pool) = self.db_pool {
			match BondsRepository::create_crawl_run(
				pool,
				&self.config.tbank_url,
				self.config.headless_chrome,
			)
			.await
			{
				Ok(id) => {
					info!("Crawl run created in DB: {}", id);
					self.run_id = Some(id);
				}
				Err(e) => {
					warn!("Failed to create crawl run in DB: {}", e);
				}
			}
		}

		self.initialize().await?;
		self.navigate_to_bonds().await?;
		self.wait_for_login().await?;

		let start_time = std::time::Instant::now();
		let mut total_bonds = Vec::new();

		loop {
			match self.check_page_available().await {
				Ok(true) => if let Ok(bonds) = self.collect_bonds().await {
						total_bonds.extend(bonds);
				},
				Ok(false) => {}
				Err(_) => {}
			}

			if let Some(duration) = duration_minutes {
				if start_time.elapsed().as_secs() > duration * 60 {
					break;
				}
			}

			// Wait for the next poll tick, but break early on Ctrl+C so an
			// interrupted crawl still falls through to finish_crawl_run below
			// and is marked 'completed' (otherwise the run stays 'running' and
			// its bonds stay hidden behind the latest-bonds view filter).
			tokio::select! {
				_ = sleep(Duration::from_secs(self.config.poll_interval_seconds)) => {}
				result = tokio::signal::ctrl_c() => {
					match result {
						Ok(()) => info!("Received Ctrl+C — finishing crawl run gracefully"),
						Err(e) => warn!("Failed to listen for Ctrl+C ({}); finishing crawl run", e),
					}
					break;
				}
			}
		}

		// Mark the run as completed in the DB
		if let (Some(ref pool), Some(run_id)) = (&self.db_pool, self.run_id) {
			let bonds_count = total_bonds.len() as i32;
			if let Err(e) =
				BondsRepository::finish_crawl_run(pool, run_id, bonds_count, "completed", None)
					.await
			{
				warn!("Failed to finish crawl run {} in DB: {}", run_id, e);
			} else {
				info!("Crawl run {} finished: {} bonds", run_id, bonds_count);
			}
		}

		Ok(total_bonds)
	}

	pub async fn close(&mut self) -> Result<()> {
		if let Some(driver) = self.driver.take() {
			driver.quit().await?;
		}
		Ok(())
	}
}

async fn try_click_show_more(driver: &WebDriver) -> Result<bool> {
	let show_more_selectors = vec!["[data-qa=\"show-more\"]", "[data-testid=\"show-more\"]"];

	for selector in show_more_selectors {
		match driver.find_all(thirtyfour::By::Css(selector)).await {
			Ok(buttons) if !buttons.is_empty() => {
				let button = &buttons[0];
				if button.is_displayed().await? {
					button.click().await?;
					sleep(Duration::from_secs(1)).await;
					return Ok(true);
				}
			}
			_ => continue,
		}
	}

	let pagination_selectors = vec![
		"a[data-qa-type=\"uikit/pagination.arrowRight\"]",
		"[data-qa=\"pagination-next\"]",
	];

	for selector in pagination_selectors {
		match driver.find_all(thirtyfour::By::Css(selector)).await {
			Ok(buttons) if !buttons.is_empty() => {
				let button = &buttons[0];
				if button.is_displayed().await? {
					button.click().await?;
					sleep(Duration::from_secs(1)).await;
					return Ok(true);
				}
			}
			_ => continue,
		}
	}

	Ok(false)
}

impl Drop for BondsCrawler {
	fn drop(&mut self) {
		if self.driver.is_some() {
			let driver = self.driver.take();
			tokio::spawn(async move {
				if let Some(d) = driver {
					let _ = d.quit().await;
				}
			});
		}
	}
}
