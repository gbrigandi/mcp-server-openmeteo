use chrono::NaiveDate;
use rmcp::{
    model::{
        CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    schemars, tool,
    transport::stdio,
    Error as McpError, ServerHandler, ServiceExt,
};
use serde_json::Value;
use std::env;
use std::sync::Arc;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct GetCurrentWeatherParams {
    #[schemars(description = "Latitude coordinate (-90 to 90)")]
    latitude: f64,
    #[schemars(description = "Longitude coordinate (-180 to 180)")]
    longitude: f64,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct GetWeatherForecastParams {
    #[schemars(description = "Latitude coordinate (-90 to 90)")]
    latitude: f64,
    #[schemars(description = "Longitude coordinate (-180 to 180)")]
    longitude: f64,
    #[schemars(description = "Number of forecast days (1-16, default: 7)")]
    days: Option<u32>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct GetHistoricalWeatherParams {
    #[schemars(description = "Latitude coordinate (-90 to 90)")]
    latitude: f64,
    #[schemars(description = "Longitude coordinate (-180 to 180)")]
    longitude: f64,
    #[schemars(description = "Start date (YYYY-MM-DD)")]
    start_date: String,
    #[schemars(description = "End date (YYYY-MM-DD)")]
    end_date: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct SearchLocationsParams {
    #[schemars(description = "Location search query in format 'city, country' (country is optional). Examples: 'Paris, France', 'Tokyo', 'New York, USA'")]
    query: String,
    #[schemars(description = "Maximum number of results (default: 10)")]
    limit: Option<u32>,
}

#[derive(Clone)]
struct OpenMeteoServer {
    client: Arc<reqwest::Client>,
}

impl OpenMeteoServer {
    fn new() -> Result<Self, anyhow::Error> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            client: Arc::new(client),
        })
    }

    fn validate_coordinates(&self, latitude: f64, longitude: f64) -> Result<(), String> {
        if latitude < -90.0 || latitude > 90.0 {
            return Err(format!(
                "Invalid latitude: {}. Must be between -90 and 90.",
                latitude
            ));
        }
        if longitude < -180.0 || longitude > 180.0 {
            return Err(format!(
                "Invalid longitude: {}. Must be between -180 and 180.",
                longitude
            ));
        }
        Ok(())
    }

    fn validate_date(&self, date_str: &str) -> Result<NaiveDate, String> {
        NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .map_err(|_| format!("Invalid date format: '{}'. Expected YYYY-MM-DD.", date_str))
    }

    async fn fetch_current_weather(
        &self,
        latitude: f64,
        longitude: f64,
    ) -> Result<Value, anyhow::Error> {
        let url = format!(
            "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,relative_humidity_2m,apparent_temperature,is_day,precipitation,rain,showers,snowfall,weather_code,cloud_cover,pressure_msl,surface_pressure,wind_speed_10m,wind_direction_10m,wind_gusts_10m",
            latitude, longitude, 
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "OpenMeteo API error: {}",
                response.status()
            ));
        }

        let data: Value = response.json().await?;
        Ok(data)
    }

    async fn fetch_weather_forecast(
        &self,
        latitude: f64,
        longitude: f64,
        days: u32,
    ) -> Result<Value, anyhow::Error> {
        let url = format!(
            "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&daily=weather_code,temperature_2m_max,temperature_2m_min,apparent_temperature_max,apparent_temperature_min,sunrise,sunset,daylight_duration,sunshine_duration,uv_index_max,precipitation_sum,rain_sum,showers_sum,snowfall_sum,precipitation_hours,precipitation_probability_max,wind_speed_10m_max,wind_gusts_10m_max,wind_direction_10m_dominant,shortwave_radiation_sum&forecast_days={}",
            latitude, longitude, days
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "OpenMeteo API error: {}",
                response.status()
            ));
        }

        let data: Value = response.json().await?;
        Ok(data)
    }

    async fn fetch_historical_weather(
        &self,
        latitude: f64,
        longitude: f64,
        start_date: &str,
        end_date: &str,
    ) -> Result<Value, anyhow::Error> {
        let url = format!(
            "https://api.open-meteo.com/v1/archive?latitude={}&longitude={}&start_date={}&end_date={}&daily=weather_code,temperature_2m_max,temperature_2m_min,temperature_2m_mean,apparent_temperature_max,apparent_temperature_min,apparent_temperature_mean,sunrise,sunset,daylight_duration,sunshine_duration,precipitation_sum,rain_sum,snowfall_sum,precipitation_hours,wind_speed_10m_max,wind_gusts_10m_max,wind_direction_10m_dominant",
            latitude, longitude, start_date, end_date 
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "OpenMeteo API error: {}",
                response.status()
            ));
        }

        let data: Value = response.json().await?;
        Ok(data)
    }

    async fn search_locations_helper(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Value, anyhow::Error> {
        let url = format!(
            "https://geocoding-api.open-meteo.com/v1/search?name={}&count={}&language=en&format=json",
            urlencoding::encode(query), limit
        );
        tracing::debug!("Geocoding API URL: {}", url); // Log the URL

        let response = self.client.get(&url).send().await?;

        let status = response.status();
        tracing::debug!("Geocoding API response status: {}", status);

        if !status.is_success() {
            let err_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error body".to_string());
            tracing::error!(
                "Geocoding API non-success. Status: {}. Body: {}",
                status,
                err_text
            );
            return Err(anyhow::anyhow!(
                "OpenMeteo Geocoding API error: {}. Body: {}",
                status,
                err_text
            ));
        }

        let response_text = response.text().await?;
        tracing::debug!("Geocoding API response text: {}", response_text);

        let data: Value = serde_json::from_str(&response_text).map_err(|e| {
            tracing::error!(
                "Failed to parse Geocoding API JSON. Error: {}. Response text: {}",
                e,
                response_text
            );
            anyhow::anyhow!(
                "Failed to parse Geocoding API JSON response: {}. Response text snippet: {:.200}",
                e,
                response_text
            )
        })?;

        Ok(data)
    }

    fn format_current_weather(&self, data: &Value, latitude: f64, longitude: f64) -> String {
        let current = data.get("current").unwrap_or(&Value::Null);
        let current_units = data.get("current_units").unwrap_or(&Value::Null);

        let temperature = current
            .get("temperature_2m")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let temp_unit = current_units
            .get("temperature_2m")
            .and_then(|v| v.as_str())
            .unwrap_or("°C");

        let humidity = current
            .get("relative_humidity_2m")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let humidity_unit = current_units
            .get("relative_humidity_2m")
            .and_then(|v| v.as_str())
            .unwrap_or("%");

        let feels_like = current
            .get("apparent_temperature")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let precipitation = current
            .get("precipitation")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let precip_unit = current_units
            .get("precipitation")
            .and_then(|v| v.as_str())
            .unwrap_or("mm");

        let wind_speed = current
            .get("wind_speed_10m")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let wind_unit = current_units
            .get("wind_speed_10m")
            .and_then(|v| v.as_str())
            .unwrap_or("km/h");
        let wind_direction = current
            .get("wind_direction_10m")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let pressure = current
            .get("pressure_msl")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let pressure_unit = current_units
            .get("pressure_msl")
            .and_then(|v| v.as_str())
            .unwrap_or("hPa");

        let cloud_cover = current
            .get("cloud_cover")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let weather_code = current
            .get("weather_code")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let is_day = current.get("is_day").and_then(|v| v.as_u64()).unwrap_or(0) == 1;

        let time = current
            .get("time")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");

        let weather_description = self.get_weather_description(weather_code, is_day);

        format!(
            "🌍 Current Weather\nLocation: {:.2}°, {:.2}°\nTime: {}\n\n🌡️ Temperature: {:.1}{}\n🤔 Feels like: {:.1}{}\n💧 Humidity: {:.0}{}\n☔ Precipitation: {:.1}{}\n💨 Wind: {:.1}{} from {}°\n🌫️ Cloud cover: {:.0}%\n📊 Pressure: {:.1}{}\n☀️ Conditions: {}",
            latitude, longitude, time,
            temperature, temp_unit,
            feels_like, temp_unit,
            humidity, humidity_unit,
            precipitation, precip_unit,
            wind_speed, wind_unit, wind_direction,
            cloud_cover,
            pressure, pressure_unit,
            weather_description
        )
    }

    fn format_weather_forecast(
        &self,
        data: &Value,
        latitude: f64,
        longitude: f64,
        days: u32,
    ) -> String {
        let daily = data.get("daily").unwrap_or(&Value::Null);
        let daily_units = data.get("daily_units").unwrap_or(&Value::Null);

        let empty_vec = vec![];
        let dates = daily
            .get("time")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);
        let temp_max = daily
            .get("temperature_2m_max")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);
        let temp_min = daily
            .get("temperature_2m_min")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);
        let weather_codes = daily
            .get("weather_code")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);
        let precipitation = daily
            .get("precipitation_sum")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);
        let wind_speed = daily
            .get("wind_speed_10m_max")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);

        let temp_unit = daily_units
            .get("temperature_2m_max")
            .and_then(|v| v.as_str())
            .unwrap_or("°C");
        let precip_unit = daily_units
            .get("precipitation_sum")
            .and_then(|v| v.as_str())
            .unwrap_or("mm");
        let wind_unit = daily_units
            .get("wind_speed_10m_max")
            .and_then(|v| v.as_str())
            .unwrap_or("km/h");

        let mut forecast = format!(
            "🌍 {}-Day Weather Forecast\nLocation: {:.2}°, {:.2}°\n\n",
            days, latitude, longitude
        );

        for i in 0..std::cmp::min(days as usize, dates.len()) {
            let date = dates[i].as_str().unwrap_or("Unknown");
            let max_temp = temp_max.get(i).and_then(|v| v.as_f64()).unwrap_or(0.0);
            let min_temp = temp_min.get(i).and_then(|v| v.as_f64()).unwrap_or(0.0);
            let code = weather_codes.get(i).and_then(|v| v.as_u64()).unwrap_or(0);
            let precip = precipitation.get(i).and_then(|v| v.as_f64()).unwrap_or(0.0);
            let wind = wind_speed.get(i).and_then(|v| v.as_f64()).unwrap_or(0.0);

            let weather_desc = self.get_weather_description(code, true); // Assume day for forecast

            forecast.push_str(&format!(
                "📅 {}\n🌡️ {:.1}{} / {:.1}{}\n☀️ {}\n☔ {:.1}{}\n💨 {:.1}{}\n\n",
                date,
                max_temp,
                temp_unit,
                min_temp,
                temp_unit,
                weather_desc,
                precip,
                precip_unit,
                wind,
                wind_unit
            ));
        }

        forecast
    }

    fn format_historical_weather(
        &self,
        data: &Value,
        latitude: f64,
        longitude: f64,
        start_date: &str,
        end_date: &str,
    ) -> String {
        let daily = data.get("daily").unwrap_or(&Value::Null);
        let daily_units = data.get("daily_units").unwrap_or(&Value::Null);

        let empty_vec = vec![];
        let dates = daily
            .get("time")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);
        let temp_max = daily
            .get("temperature_2m_max")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);
        let temp_min = daily
            .get("temperature_2m_min")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);
        let temp_mean = daily
            .get("temperature_2m_mean")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);
        let precipitation = daily
            .get("precipitation_sum")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);

        let temp_unit = daily_units
            .get("temperature_2m_max")
            .and_then(|v| v.as_str())
            .unwrap_or("°C");
        let precip_unit = daily_units
            .get("precipitation_sum")
            .and_then(|v| v.as_str())
            .unwrap_or("mm");

        let mut history = format!(
            "🌍 Historical Weather Data\nLocation: {:.2}°, {:.2}°\nPeriod: {} to {}\n\n",
            latitude, longitude, start_date, end_date
        );

        let mut total_temp_max = 0.0;
        let mut total_temp_min = 0.0;
        let mut total_temp_mean = 0.0;
        let mut total_precip = 0.0;
        let mut count = 0;

        for i in 0..dates.len() {
            if let (Some(max_temp), Some(min_temp), Some(mean_temp), Some(precip)) = (
                temp_max.get(i).and_then(|v| v.as_f64()),
                temp_min.get(i).and_then(|v| v.as_f64()),
                temp_mean.get(i).and_then(|v| v.as_f64()),
                precipitation.get(i).and_then(|v| v.as_f64()),
            ) {
                total_temp_max += max_temp;
                total_temp_min += min_temp;
                total_temp_mean += mean_temp;
                total_precip += precip;
                count += 1;
            }
        }

        if count > 0 {
            history.push_str(&format!(
                "📊 Summary Statistics ({} days):\n🌡️ Average High: {:.1}{}\n🌡️ Average Low: {:.1}{}\n🌡️ Average Mean: {:.1}{}\n☔ Total Precipitation: {:.1}{}\n☔ Average Daily Precipitation: {:.1}{}\n\n",
                count,
                total_temp_max / count as f64, temp_unit,
                total_temp_min / count as f64, temp_unit,
                total_temp_mean / count as f64, temp_unit,
                total_precip, precip_unit,
                total_precip / count as f64, precip_unit
            ));
        }

        history.push_str("📅 Daily Data (first 5 days):\n");
        for i in 0..std::cmp::min(5, dates.len()) {
            let date = dates[i].as_str().unwrap_or("Unknown");
            let max_temp = temp_max.get(i).and_then(|v| v.as_f64()).unwrap_or(0.0);
            let min_temp = temp_min.get(i).and_then(|v| v.as_f64()).unwrap_or(0.0);
            let precip = precipitation.get(i).and_then(|v| v.as_f64()).unwrap_or(0.0);

            history.push_str(&format!(
                "{}: {:.1}{} / {:.1}{}, {:.1}{}\n",
                date, max_temp, temp_unit, min_temp, temp_unit, precip, precip_unit
            ));
        }

        history
    }

    fn format_locations(&self, data: &Value) -> String {
        let empty_vec = vec![];
        let results = data
            .get("results")
            .and_then(|v| v.as_array())
            .unwrap_or(&empty_vec);

        if results.is_empty() {
            return "No locations found matching your search query.".to_string();
        }

        let mut locations = "🌍 Location Search Results:\n\n".to_string();

        for (i, result) in results.iter().enumerate() {
            let name = result
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");
            let country = result
                .get("country")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");
            let admin1 = result.get("admin1").and_then(|v| v.as_str());
            let latitude = result
                .get("latitude")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let longitude = result
                .get("longitude")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let timezone = result
                .get("timezone")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");
            let population = result.get("population").and_then(|v| v.as_u64());

            let admin_info = if let Some(admin) = admin1 {
                format!(", {}", admin)
            } else {
                String::new()
            };

            let pop_info = if let Some(pop) = population {
                format!("\n👥 Population: {}", pop)
            } else {
                String::new()
            };

            locations.push_str(&format!(
                "{}. 📍 {}{}, {}\n📍 Coordinates: {:.4}°, {:.4}°\n🕐 Timezone: {}{}\n\n",
                i + 1,
                name,
                admin_info,
                country,
                latitude,
                longitude,
                timezone,
                pop_info
            ));
        }

        locations
    }

    fn get_weather_description(&self, code: u64, _is_day: bool) -> &'static str {
        match code {
            0 => "Clear sky",
            1 => "Mainly clear",
            2 => "Partly cloudy",
            3 => "Overcast",
            45 => "Fog",
            48 => "Depositing rime fog",
            51 => "Light drizzle",
            53 => "Moderate drizzle",
            55 => "Dense drizzle",
            56 => "Light freezing drizzle",
            57 => "Dense freezing drizzle",
            61 => "Slight rain",
            63 => "Moderate rain",
            65 => "Heavy rain",
            66 => "Light freezing rain",
            67 => "Heavy freezing rain",
            71 => "Slight snow fall",
            73 => "Moderate snow fall",
            75 => "Heavy snow fall",
            77 => "Snow grains",
            80 => "Slight rain showers",
            81 => "Moderate rain showers",
            82 => "Violent rain showers",
            85 => "Slight snow showers",
            86 => "Heavy snow showers",
            95 => "Thunderstorm",
            96 => "Thunderstorm with slight hail",
            99 => "Thunderstorm with heavy hail",
            _ => "Unknown conditions",
        }
    }
}

#[tool(tool_box)]
impl OpenMeteoServer {
    #[tool(
        name = "get_current_weather",
        description = "Get current weather conditions for a specific location. Returns real-time weather data including temperature, humidity, precipitation, wind, and atmospheric conditions."
    )]
    async fn get_current_weather(
        &self,
        #[tool(aggr)] params: GetCurrentWeatherParams,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!(
            latitude = %params.latitude,
            longitude = %params.longitude,
            "Getting current weather"
        );

        if let Err(err) = self.validate_coordinates(params.latitude, params.longitude) {
            tracing::error!("Invalid coordinates: {}", err);
            return Ok(CallToolResult::error(vec![Content::text(err)]));
        }


        match self
            .fetch_current_weather(params.latitude, params.longitude)
            .await
        {
            Ok(data) => {
                let formatted =
                    self.format_current_weather(&data, params.latitude, params.longitude);
                tracing::info!("Successfully retrieved current weather");
                Ok(CallToolResult::success(vec![Content::text(formatted)]))
            }
            Err(e) => {
                let err_msg = format!("Error retrieving current weather: {}", e);
                tracing::error!("{}", err_msg);
                Ok(CallToolResult::error(vec![Content::text(err_msg)]))
            }
        }
    }

    #[tool(
        name = "get_weather_forecast",
        description = "Get weather forecast for a specific location. Returns detailed forecast data for up to 16 days including daily temperature, precipitation, wind, and weather conditions."
    )]
    async fn get_weather_forecast(
        &self,
        #[tool(aggr)] params: GetWeatherForecastParams,
    ) -> Result<CallToolResult, McpError> {
        let days = params.days.unwrap_or(7).clamp(1, 16);

        tracing::info!(
            latitude = %params.latitude,
            longitude = %params.longitude,
            days = %days,
            "Getting weather forecast"
        );

        if let Err(err) = self.validate_coordinates(params.latitude, params.longitude) {
            tracing::error!("Invalid coordinates: {}", err);
            return Ok(CallToolResult::error(vec![Content::text(err)]));
        }

        match self
            .fetch_weather_forecast(params.latitude, params.longitude, days)
            .await
        {
            Ok(data) => {
                let formatted =
                    self.format_weather_forecast(&data, params.latitude, params.longitude, days);
                tracing::info!("Successfully retrieved weather forecast for {} days", days);
                Ok(CallToolResult::success(vec![Content::text(formatted)]))
            }
            Err(e) => {
                let err_msg = format!("Error retrieving weather forecast: {}", e);
                tracing::error!("{}", err_msg);
                Ok(CallToolResult::error(vec![Content::text(err_msg)]))
            }
        }
    }

    #[tool(
        name = "get_historical_weather",
        description = "Get historical weather data for a specific location and date range. Returns daily weather statistics including temperature, precipitation, and other meteorological data for analysis."
    )]
    async fn get_historical_weather(
        &self,
        #[tool(aggr)] params: GetHistoricalWeatherParams,
    ) -> Result<CallToolResult, McpError> {
        tracing::info!(
            latitude = %params.latitude,
            longitude = %params.longitude,
            start_date = %params.start_date,
            end_date = %params.end_date,
            "Getting historical weather"
        );

        if let Err(err) = self.validate_coordinates(params.latitude, params.longitude) {
            tracing::error!("Invalid coordinates: {}", err);
            return Ok(CallToolResult::error(vec![Content::text(err)]));
        }

        if let Err(err) = self.validate_date(&params.start_date) {
            tracing::error!("Invalid start date: {}", err);
            return Ok(CallToolResult::error(vec![Content::text(err)]));
        }

        if let Err(err) = self.validate_date(&params.end_date) {
            tracing::error!("Invalid end date: {}", err);
            return Ok(CallToolResult::error(vec![Content::text(err)]));
        }

        match self
            .fetch_historical_weather(
                params.latitude,
                params.longitude,
                &params.start_date,
                &params.end_date,
            )
            .await
        {
            Ok(data) => {
                let formatted = self.format_historical_weather(
                    &data,
                    params.latitude,
                    params.longitude,
                    &params.start_date,
                    &params.end_date,
                );
                tracing::info!("Successfully retrieved historical weather data");
                Ok(CallToolResult::success(vec![Content::text(formatted)]))
            }
            Err(e) => {
                let err_msg = format!("Error retrieving historical weather: {}", e);
                tracing::error!("{}", err_msg);
                Ok(CallToolResult::error(vec![Content::text(err_msg)]))
            }
        }
    }

    #[tool(
        name = "search_locations",
        description = "Search for locations by name to get their coordinates and details. Use format 'city, country' where country is optional (e.g., 'Paris, France' or just 'Tokyo'). Returns a list of matching locations with coordinates and other geographic information."
    )]
    async fn search_locations(
        &self,
        #[tool(aggr)] params: SearchLocationsParams,
    ) -> Result<CallToolResult, McpError> {
        let limit = params.limit.unwrap_or(10).clamp(1, 100);

        tracing::info!(
            query = %params.query,
            limit = %limit,
            "Searching locations"
        );

        match self.search_locations_helper(&params.query, limit).await {
            Ok(data) => {
                let formatted = self.format_locations(&data);
                tracing::info!("Successfully searched locations");
                Ok(CallToolResult::success(vec![Content::text(formatted)]))
            }
            Err(e) => {
                let err_msg = format!("Error searching locations: {}", e);
                tracing::error!("{}", err_msg);
                Ok(CallToolResult::error(vec![Content::text(err_msg)]))
            }
        }
    }
}

#[tool(tool_box)]
impl ServerHandler for OpenMeteoServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_prompts()
                .enable_resources()
                .enable_tools()
                .build(),
            server_info: Implementation {
                name: env!("CARGO_PKG_NAME").to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some(
                "This server provides tools to interact with the OpenMeteo Weather API for weather data and forecasts.\n\
                Available tools:\n\
                - 'get_current_weather': Get current weather conditions for a specific location. \
                Requires 'latitude' and 'longitude' parameters.\n\
                - 'get_weather_forecast': Get weather forecast for a specific location. \
                Requires 'latitude' and 'longitude' parameters. Optional 'days' parameter (1-16, defaults to 7).\n\
                - 'get_historical_weather': Get historical weather data for a specific location and date range. \
                Requires 'latitude', 'longitude', 'start_date', and 'end_date' parameters (dates in YYYY-MM-DD format).\n\
                - 'search_locations': Search for locations by name to get their coordinates. \
                Requires 'query' parameter in format 'city, country' (country is optional, e.g., 'Paris, France' or 'Tokyo'). \
                Optional 'limit' parameter (defaults to 10, max 100).\n\n\
                Coordinates must be valid: latitude between -90 and 90, longitude between -180 and 180.\n\
                All weather data is provided by OpenMeteo (https://open-meteo.com/) and is free to use."
                    .to_string(),
            ),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::DEBUG.into()),
        )
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("Starting OpenMeteo MCP Server...");

    // Create an instance of our OpenMeteo server
    let server = OpenMeteoServer::new().expect("Error initializing OpenMeteo server");

    tracing::info!("Using stdio transport");
    let service = server.serve(stdio()).await.inspect_err(|e| {
        tracing::error!("serving error: {:?}", e);
    })?;

    service.waiting().await?;
    Ok(())
}
