use std::collections::HashMap;
use reqwest::{Response, StatusCode};

pub struct WeatherUnion {
    api_key: String,
}

#[derive(serde::Deserialize)]
struct BodyValues {
    message: String,
    locality_weather_data: HashMap<String, Option<f64>>,
    device_type: u8
}

#[derive(Debug)]
pub struct LocalityWeatherData {
    device: u8,
    temperature: f64,
    humidity: f64,
    wind_speed: f64,
    wind_direction: f64,
    rain_intensity: f64,
    rain_accumulation: f64,
}

#[derive(Debug)]
pub enum WeatherResponseError {
    ErrorRetrievingData, NotSupported, ApiKeyLimitExhausted, CouldNotAuthenticate, TemporarilyUnavailable(String), UnknownError(StatusCode), InvalidResponse
}

impl WeatherUnion {

    fn from_key(key: String) -> WeatherUnion {
        return WeatherUnion {api_key: key}
    }

    async fn process_payload(&self, payload: Response) -> Result<LocalityWeatherData, WeatherResponseError> {
        match payload.status() {
            // 200, successful response
            StatusCode::OK => {
                let body = payload.text().await.unwrap();
                let parsed = if serde_json::from_str::<BodyValues>(body.as_str()).is_ok()
                { serde_json::from_str::<BodyValues>(body.as_str()).unwrap() }
                else{ return Err(WeatherResponseError::InvalidResponse) };
                return if !parsed.message.is_empty() {
                    Err(WeatherResponseError::TemporarilyUnavailable(parsed.message))
                } else {
                    Ok(LocalityWeatherData {
                        device: parsed.device_type,
                        temperature:
                        if parsed.locality_weather_data.get("temperature").is_some() &&
                            parsed.locality_weather_data.get("temperature").unwrap().is_some()
                        { parsed.locality_weather_data.get("temperature").unwrap().unwrap() } else { 0.0 },
                        humidity: if parsed.locality_weather_data.get("humidity").is_some() &&
                            parsed.locality_weather_data.get("humidity").unwrap().is_some()
                        { parsed.locality_weather_data.get("humidity").unwrap().unwrap() } else { 0.0 },
                        wind_speed: if parsed.locality_weather_data.get("wind_speed").is_some() &&
                            parsed.locality_weather_data.get("wind_speed").unwrap().is_some()
                        { parsed.locality_weather_data.get("wind_speed").unwrap().unwrap() } else { 0.0 },
                        wind_direction: if parsed.locality_weather_data.get("wind_direction").is_some() &&
                            parsed.locality_weather_data.get("wind_direction").unwrap().is_some()
                        { parsed.locality_weather_data.get("wind_direction").unwrap().unwrap() } else { 0.0 },
                        rain_intensity: if parsed.locality_weather_data.get("rain_intensity").is_some() &&
                            parsed.locality_weather_data.get("rain_intensity").unwrap().is_some()
                        { parsed.locality_weather_data.get("rain_intensity").unwrap().unwrap() } else { 0.0 },
                        rain_accumulation: if parsed.locality_weather_data.get("rain_accumulation").is_some()
                            && parsed.locality_weather_data.get("rain_accumulation").unwrap().is_some()
                        { parsed.locality_weather_data.get("rain_accumulation").unwrap().unwrap() } else { 0.0 },
                    })
                }

            }
            // 500, error retrieving data
            StatusCode::INTERNAL_SERVER_ERROR => {
                return Err(WeatherResponseError::ErrorRetrievingData)}
            // 400, latitude longitude / locality id not supported
            StatusCode::BAD_REQUEST => {
                return Err(WeatherResponseError::NotSupported)
            }
            // 429, api key limit exhausted
            StatusCode::TOO_MANY_REQUESTS => {
                return Err(WeatherResponseError::ApiKeyLimitExhausted)
            }
            // 403, could not authenticate
            StatusCode::FORBIDDEN => {
                return Err(WeatherResponseError::CouldNotAuthenticate)
            }
            other => {
                return Err(WeatherResponseError::UnknownError(other))
            }
        }
    }

    pub async fn lat_long(&self, lat: f64, long: f64) -> Result<LocalityWeatherData, WeatherResponseError> {
        let client = reqwest::Client::new(); // create new client every request as we dont need to save data
        let response = client.get(format!(
            "https://www.weatherunion.com/gw/weather/external/v0/get_weather_data?\
                latitude={lat}&longitude={long}"
            )).header("x-zomato-api-key", &self.api_key).send().await.unwrap();
        drop(client);
        return self.process_payload(response).await;
    }

    pub async fn locality_id(&self, id: String) -> Result<LocalityWeatherData, WeatherResponseError> {
        let client = reqwest::Client::new(); // create new client every request as we dont need to save data
        let response = client.get(format!(
            "https://www.weatherunion.com/gw/weather/external/v0/get_locality_weather_data?locality_id={id}"
        )).header("x-zomato-api-key", &self.api_key).send().await.unwrap();
        drop(client);
        return self.process_payload(response).await;
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use super::*;
    macro_rules! aw {
    ($e:expr) => {
        tokio_test::block_on($e)
    };
  }
    #[test]
    fn test_lat_long() {
        let api_key = fs::read_to_string("target/api_key")
            .expect("Should have been able to read the file");
        let variable = WeatherUnion::from_key(api_key);
        let out = aw!(variable.lat_long(12.936787, 77.556079)); // Banashankari, BLR
        drop(variable);
        println!("lat_long {:?}", out);
        assert!(out.is_ok());
    }

    #[test]
    fn test_locality_id() {
        let api_key = fs::read_to_string("target/api_key")
            .expect("Should have been able to read the file");
        let variable = WeatherUnion::from_key(api_key);
        let out = aw!(variable.locality_id("ZWL003467".to_string())); // Banashankari, BLR
        drop(variable);
        println!("locality_id {:?}", out);
        assert!(out.is_ok());
    }

    #[test]
    fn test_locality_rgs() {
        let api_key = fs::read_to_string("target/api_key")
            .expect("Should have been able to read the file");
        let variable = WeatherUnion::from_key(api_key);
        let out = aw!(variable.locality_id("ZWL008436".to_string())); // Moudhapara, Raipur
        drop(variable);
        println!("locality_id_rgs {:?}", out);
        assert!(out.is_ok());

    }
}
