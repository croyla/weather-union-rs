extern crate core;

use core::fmt;
use std::collections::HashMap;
use std::fmt::Formatter;
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

#[derive(Clone, Copy, Debug)]
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
        return match payload.status() {
            // 200, successful response
            StatusCode::OK => {
                let body = payload.text().await.unwrap();
                let parsed = if serde_json::from_str::<BodyValues>(body.as_str()).is_ok()
                { serde_json::from_str::<BodyValues>(body.as_str()).unwrap() } else { return Err(WeatherResponseError::InvalidResponse) };
                if !parsed.message.is_empty() {
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
                Err(WeatherResponseError::ErrorRetrievingData)
            }
            // 400, latitude longitude / locality id not supported
            StatusCode::BAD_REQUEST => {
                Err(WeatherResponseError::NotSupported)
            }
            // 429, api key limit exhausted
            StatusCode::TOO_MANY_REQUESTS => {
                Err(WeatherResponseError::ApiKeyLimitExhausted)
            }
            // 403, could not authenticate
            StatusCode::FORBIDDEN => {
                Err(WeatherResponseError::CouldNotAuthenticate)
            }
            other => {
                Err(WeatherResponseError::UnknownError(other))
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

    pub async fn locality(&self, id: LocalityId) -> Result<LocalityWeatherData, WeatherResponseError> {
        let client = reqwest::Client::new(); // create new client every request as we dont need to save data
        let response = client.get(format!(
            "https://www.weatherunion.com/gw/weather/external/v0/get_locality_weather_data?locality_id={}", id.0
        )).header("x-zomato-api-key", &self.api_key).send().await.unwrap();
        drop(client);
        return self.process_payload(response).await;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LocalityId(&'static str);

pub struct InvalidLocalityId {
    _priv: (),
}
impl InvalidLocalityId {
    fn new() -> InvalidLocalityId {
        InvalidLocalityId { _priv: () }
    }
}
impl LocalityId {

    pub fn from_str(id: &str) -> Result<LocalityId, InvalidLocalityId> {
        if id.is_empty() {
            return Err(InvalidLocalityId::new())
        }
        return from_str(id).ok_or(InvalidLocalityId::new())

    }

    pub fn locality_name(&self) -> Option<&str> {
        return area_name(self.0)
    }
    pub fn locality_lat_long(&self) -> Option<(f64, f64)> {
        return area_lat_long(self.0)
    }
}

impl fmt::Display for LocalityId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {}",
            self.0,
            self.locality_name().unwrap_or("<Unknown LocalityId>")
        )
    }
}

macro_rules! locality_id {
    (
        $(
        ($str:expr, $konst:ident, $area:expr, $lat_long:expr);
        )+
    ) => {
        impl LocalityId {
            $(
            pub const $konst: LocalityId = LocalityId($str);
            )+
        }
        fn area_name(id: &str) -> Option<&str>{
            match id {
                $(
                $str => Some($area),
                )+
                _ => None
            }
        }
        fn area_lat_long(id: &str) -> Option<(f64, f64)>{
            match id {
                $(
                $str => Some($lat_long),
                )+
                _ => None
            }
        }
        fn from_str(id: &str) -> Option<LocalityId>{
            match id {
                $(
                $str => Some(LocalityId::$konst),
                )+
                _ => None
            }
        }
        // fn from_lat_long(lat_long: (f64, f64)) -> Option<LocalityId>{
        //     match lat_long {
        //         $(
        //         $lat_long => Some(LocalityId::$konst),
        //         )+
        //         _ => None
        //     }
        // }
    };
}

locality_id! {
    ("ZWL005764", ZWL005764, "Delhi NCR Sarita Vihar", (28.531759, 77.293973));
    ("ZWL008752", ZWL008752, "Delhi NCR Faridabad Sector 41-50", (28.460895, 77.304764));
    ("ZWL005996", ZWL005996, "Delhi NCR New Friends Colony", (28.565268, 77.274971));
    ("ZWL005243", ZWL005243, "Delhi NCR Sector 26 (Noida)", (28.574404, 77.334178));
    ("ZWL009032", ZWL009032, "Delhi NCR New Industrial Town", (28.375702, 77.299442));
    ("ZWL005428", ZWL005428, "Delhi NCR Tilak Nagar", (28.641372, 77.094689));
    ("ZWL001073", ZWL001073, "Delhi NCR Sector 10, Gurgaon", (28.436077, 76.996757));
    ("ZWL001319", ZWL001319, "Delhi NCR Ashok Vihar, Delhi", (28.684453, 77.174418));
    ("ZWL004800", ZWL004800, "Delhi NCR Kalkaji", (28.529029, 77.258939));
    ("ZWL003118", ZWL003118, "Delhi NCR Sector 53", (28.442743, 77.104379));
    ("ZWL002091", ZWL002091, "Delhi NCR Sector 49", (28.408012, 77.050064));
    ("ZWL002662", ZWL002662, "Delhi NCR Vasundhara", (28.665225, 77.366782));
    ("ZWL001404", ZWL001404, "Delhi NCR Rajinder Nagar", (28.640732, 77.182900));
    ("ZWL008963", ZWL008963, "Delhi NCR Safdarjung Enclave", (28.562608, 77.191457));
    ("ZWL006538", ZWL006538, "Delhi NCR Connaught Place", (28.630630, 77.220640));
    ("ZWL003075", ZWL003075, "Delhi NCR Sector 66", (28.380856, 77.062751));
    ("ZWL003721", ZWL003721, "Delhi NCR Sector 57", (28.422100, 77.082740));
    ("ZWL006396", ZWL006396, "Delhi NCR Moti Bagh, Delhi", (28.575774, 77.180697));
    ("ZWL004535", ZWL004535, "Delhi NCR Patel Nagar, Delhi", (28.652848, 77.167909));
    ("ZWL008554", ZWL008554, "Delhi NCR Greater Noida", (28.456171, 77.521577));
    ("ZWL004533", ZWL004533, "Delhi NCR Karkardooma, Delhi", (28.656829, 77.310553));
    ("ZWL002179", ZWL002179, "Delhi NCR Tigaon", (28.417120, 77.412569));
    ("ZWL007487", ZWL007487, "Delhi NCR Sector 50 (Noida)", (28.569103, 77.364876));
    ("ZWL007120", ZWL007120, "Delhi NCR Vasant Kunj, Delhi", (28.524633, 77.151206));
    ("ZWL007486", ZWL007486, "Delhi NCR Dwarka, Delhi", (28.594467, 77.047747));
    ("ZWL006287", ZWL006287, "Delhi NCR Sector 15", (28.457927, 77.034816));
    ("ZWL002146", ZWL002146, "Delhi NCR Mayur Vihar Phase III", (28.606000, 77.323675));
    ("ZWL008405", ZWL008405, "Delhi NCR Crossing Republik", (28.635043, 77.419056));
    ("ZWL004455", ZWL004455, "Delhi NCR Sector 28", (28.473457, 77.087532));
    ("ZWL005087", ZWL005087, "Delhi NCR Palam Vihar, Gurgaon", (28.508782, 77.033506));
    ("ZWL009648", ZWL009648, "Delhi NCR Sector 63 (Noida)", (28.621672, 77.386474));
    ("ZWL008317", ZWL008317, "Delhi NCR Raj Nagar, Ghaziabad", (28.689174, 77.428976));
    ("ZWL005878", ZWL005878, "Delhi NCR Sector 128", (28.526706, 77.354868));
    ("ZWL003241", ZWL003241, "Delhi NCR Sector 56, Gurgaon", (28.418235, 77.101860));
    ("ZWL007224", ZWL007224, "Delhi NCR Indirapuram", (28.644059, 77.373883));
    ("ZWL009834", ZWL009834, "Delhi NCR Malviya Nagar", (28.536048, 77.213453));
    ("ZWL007284", ZWL007284, "Delhi NCR Sector 43, Gurgaon", (28.454416, 77.088820));
    ("ZWL006738", ZWL006738, "Delhi NCR Sector 120 (Noida)", (28.586854, 77.390832));
    ("ZWL007329", ZWL007329, "Delhi NCR Saket", (28.523171, 77.207543));
    ("ZWL001752", ZWL001752, "Delhi NCR Sector 18 (Noida)", (28.568937, 77.324414));
    ("ZWL007594", ZWL007594, "Delhi NCR Naraina", (28.627479, 77.142115));
    ("ZWL006116", ZWL006116, "Delhi NCR Patparganj", (28.632961, 77.308344));
    ("ZWL009925", ZWL009925, "Delhi NCR Ghitorni", (28.486412, 77.125366));
    ("ZWL009335", ZWL009335, "Delhi NCR Faridabad Sector 1-11", (28.365131, 77.326157));
    ("ZWL009638", ZWL009638, "Delhi NCR Sector 24", (28.497419, 77.090980));
    ("ZWL005670", ZWL005670, "Delhi NCR Rajouri Garden", (28.646438, 77.122357));
    ("ZWL003757", ZWL003757, "Delhi NCR Vishnu Garden", (28.646933, 77.095064));
    ("ZWL003610", ZWL003610, "Delhi NCR Sector 48, Gurgaon", (28.416008, 77.032164));
    ("ZWL005971", ZWL005971, "Delhi NCR Kirti Nagar", (28.654433, 77.142367));
    ("ZWL003626", ZWL003626, "Delhi NCR Faridabad Sector 81-89", (28.397247, 77.345569));
    ("ZWL005876", ZWL005876, "Delhi NCR GK I", (28.550911, 77.235272));
    ("ZWL006295", ZWL006295, "Delhi NCR Mohan Estate", (28.494788, 77.312727));
    ("ZWL007653", ZWL007653, "Delhi NCR Mukherjee Nagar", (28.702971, 77.209740));
    ("ZWL006697", ZWL006697, "Delhi NCR Mehrauli", (28.524426, 77.183996));
    ("ZWL003259", ZWL003259, "Delhi NCR Burari", (28.753669, 77.191037));
    ("ZWL004759", ZWL004759, "Delhi NCR Gaur city, Ghaziabad", (28.607703, 77.434385));
    ("ZWL004477", ZWL004477, "Delhi NCR GK II", (28.533936, 77.243800));
    ("ZWL005077", ZWL005077, "Delhi NCR Rohini", (28.723712, 77.104596));
    ("ZWL008191", ZWL008191, "Delhi NCR Rangpuri", (28.533976, 77.119516));
    ("ZWL006092", ZWL006092, "Delhi NCR Sector 46", (28.438586, 77.060773));
    ("ZWL001549", ZWL001549, "Delhi NCR Sector 62 (Noida)", (28.611088, 77.369652));
    ("ZWL001036", ZWL001036, "Delhi NCR Shalimar Bagh", (28.720312, 77.164849));
    ("ZWL006996", ZWL006996, "Delhi NCR Model Town", (28.717232, 77.194643));
    ("ZWL007566", ZWL007566, "Delhi NCR Faridabad Sector 16-20", (28.422437, 77.310113));
    ("ZWL009852", ZWL009852, "Delhi NCR Sector 2 (Noida)", (28.581459, 77.316720));
    ("ZWL008648", ZWL008648, "Delhi NCR Govindpuram", (28.689317, 77.486930));
    ("ZWL009728", ZWL009728, "Delhi NCR Gwal Pahari", (28.435122, 77.136308));
    ("ZWL006868", ZWL006868, "Delhi NCR Nehru Nagar", (28.653441, 77.449969));
    ("ZWL002067", ZWL002067, "Delhi NCR Chittaranjan Park", (28.537530, 77.249070));
    ("ZWL008791", ZWL008791, "Delhi NCR IMT Manesar", (28.384492, 76.941950));
    ("ZWL003043", ZWL003043, "Delhi NCR Sector 73 Z Kitchen", (28.580105, 77.385436));
    ("ZWL004159", ZWL004159, "Delhi NCR Sector 51", (28.430042, 77.065213));
    ("ZWL005960", ZWL005960, "Delhi NCR Ballabhgarh", (28.343049, 77.330317));
    ("ZWL009293", ZWL009293, "Delhi NCR Nangloi", (28.659524, 77.060800));
    ("ZWL001663", ZWL001663, "Delhi NCR Uttam Nagar", (28.616774, 77.057136));
    ("ZWL005762", ZWL005762, "Delhi NCR Sector 47", (28.424524, 77.050065));
    ("ZWL005345", ZWL005345, "Delhi NCR Paharganj", (28.645112, 77.212824));
    ("ZWL008225", ZWL008225, "Delhi NCR Sector 25", (28.484268, 77.084693));
    ("ZWL001933", ZWL001933, "Delhi NCR Pitampura", (28.688724, 77.138225));
    ("ZWL003591", ZWL003591, "Delhi NCR Shahdara", (28.688657, 77.278267));
    ("ZWL007061", ZWL007061, "Delhi NCR Sector 31", (28.442946, 77.057195));
    ("ZWL008476", ZWL008476, "Delhi NCR Sector 23", (28.509080, 77.057138));
    ("ZWL009008", ZWL009008, "Delhi NCR Sector 12 (Noida)", (28.599952, 77.343188));
    ("ZWL005323", ZWL005323, "Delhi NCR Mayur Vihar Phase II", (28.613695, 77.302775));
    ("ZWL001412", ZWL001412, "Delhi NCR Faridabad Sector 12-15", (28.394334, 77.324016));
    ("ZWL005925", ZWL005925, "Delhi NCR DLF PHASE 1 (SECTOR 26)", (28.477910, 77.103843));
    ("ZWL008716", ZWL008716, "Delhi NCR Laxmi Nagar", (28.627366, 77.279200));
    ("ZWL009339", ZWL009339, "Delhi NCR Karol Bagh", (28.647924, 77.190463));
    ("ZWL009096", ZWL009096, "Delhi NCR Chhatarpur", (28.497203, 77.171629));
    ("ZWL006720", ZWL006720, "Delhi NCR Paschim Vihar", (28.665591, 77.098478));
    ("ZWL002908", ZWL002908, "Delhi NCR Sector 1, Noida", (28.573663, 77.415427));
    ("ZWL001186", ZWL001186, "Delhi NCR South Extension I", (28.578498, 77.223627));
    ("ZWL004789", ZWL004789, "Delhi NCR Sector 18", (28.495291, 77.069729));
    ("ZWL008978", ZWL008978, "Delhi NCR Kamla Nagar", (28.676018, 77.208446));
    ("ZWL007903", ZWL007903, "Delhi NCR Janakpuri", (28.623431, 77.097814));
    ("ZWL008897", ZWL008897, "Delhi NCR Vikaspuri", (28.645655, 77.065922));
    ("ZWL007431", ZWL007431, "Delhi NCR Najafgarh", (28.607458, 76.995980));
    ("ZWL001112", ZWL001112, "Delhi NCR Mayur Vihar Phase 1", (28.609961, 77.296133));
    ("ZWL008649", ZWL008649, "Delhi NCR Sez Noida 1", (28.507448, 77.410089));
    ("ZWL006384", ZWL006384, "Delhi NCR Gulavali, Noida", (28.436941, 77.456903));
    ("ZWL007840", ZWL007840, "Delhi NCR Sector 14", (28.471738, 77.045472));
    ("ZWL002072", ZWL002072, "Delhi NCR Sector 76(Noida)", (28.570828, 77.390429));
    ("ZWL003077", ZWL003077, "Delhi NCR Green Park", (28.562981, 77.209729));
    ("ZWL005395", ZWL005395, "Delhi NCR Munirka", (28.554395, 77.172547));
    ("ZWL005729", ZWL005729, "Delhi NCR NEHRU PLACE", (28.555622, 77.250890));
    ("ZWL005736", ZWL005736, "Delhi NCR Lajpat Nagar", (28.565415, 77.247221));
    ("ZWL007212", ZWL007212, "Delhi NCR Sector 52 (Noida)", (28.595742, 77.362995));
    ("ZWL004803", ZWL004803, "Delhi NCR Sector 100 (Noida)", (28.547583, 77.370951));
    ("ZWL003444", ZWL003444, "Delhi NCR Sector 50", (28.418947, 77.059581));
    ("ZWL008293", ZWL008293, "Delhi NCR Dilshad Garden", (28.684684, 77.320986));
    ("ZWL007308", ZWL007308, "Delhi NCR Sector 29, Gurgaon", (28.459498, 77.061046));
    ("ZWL008219", ZWL008219, "Delhi NCR SUSHANT LOK 1", (28.467923, 77.076530));
    ("ZWL006234", ZWL006234, "Delhi NCR SAHIBABAD", (28.682920, 77.362827));
    ("ZWL007666", ZWL007666, "Delhi NCR Sector 45 (Noida)", (28.555596, 77.345713));
    ("ZWL002490", ZWL002490, "Delhi NCR Sector 84", (28.395433, 76.967436));
    ("ZWL007138", ZWL007138, "Delhi NCR Sector 7, Gurgaon", (28.476288, 77.013365));
    ("ZWL009706", ZWL009706, "Delhi NCR Sector 27", (28.465260, 77.085742));
    ("ZWL001267", ZWL001267, "Delhi NCR Hauz Khas", (28.551132, 77.211401));
    ("ZWL003552", ZWL003552, "Delhi NCR Jangpura", (28.583630, 77.246915));
    ("ZWL008401", ZWL008401, "Delhi NCR Sector 52, Gurgaon", (28.443858, 77.083991));
    ("ZWL001758", ZWL001758, "Delhi NCR Vaishali, Ghaziabad", (28.649375, 77.336609));
    ("ZWL003128", ZWL003128, "Kolkata Shibpur", (22.578477, 88.315675));
    ("ZWL004322", ZWL004322, "Kolkata Kalyani 1, Kolkata", (22.983813, 88.427417));
    ("ZWL002495", ZWL002495, "Kolkata Bansdroni", (22.472208, 88.357875));
    ("ZWL009257", ZWL009257, "Kolkata Bow Barracks", (22.565476, 88.360918));
    ("ZWL005435", ZWL005435, "Kolkata Baranagar, Kolkata", (22.660464, 88.374255));
    ("ZWL007041", ZWL007041, "Kolkata Sonarpur, Kolkata", (22.432033, 88.408369));
    ("ZWL002918", ZWL002918, "Kolkata Ballygunge", (22.532686, 88.362677));
    ("ZWL003388", ZWL003388, "Kolkata Sinthi, Kolkata", (22.627697, 88.380719));
    ("ZWL008806", ZWL008806, "Kolkata Salt Lake 2", (22.583560, 88.432176));
    ("ZWL008635", ZWL008635, "Kolkata Alipore", (22.536626, 88.331142));
    ("ZWL002312", ZWL002312, "Kolkata Baguihati", (22.614990, 88.426184));
    ("ZWL001499", ZWL001499, "Kolkata South Dum Dum", (22.621570, 88.408915));
    ("ZWL003315", ZWL003315, "Kolkata Purba Barisha", (22.474612, 88.320775));
    ("ZWL003794", ZWL003794, "Kolkata Jadavpur", (22.498720, 88.362540));
    ("ZWL006574", ZWL006574, "Kolkata Tollygunge", (22.498620, 88.351732));
    ("ZWL004138", ZWL004138, "Kolkata Shyam Bazar", (22.594302, 88.377284));
    ("ZWL009022", ZWL009022, "Kolkata Behala", (22.488571, 88.306630));
    ("ZWL003271", ZWL003271, "Kolkata Chandannagar, Kolkata", (22.865442, 88.367339));
    ("ZWL003951", ZWL003951, "Kolkata Barrackpore", (22.766771, 88.361552));
    ("ZWL003915", ZWL003915, "Kolkata East Kolkata Township", (22.512832, 88.399411));
    ("ZWL006750", ZWL006750, "Kolkata Bhowanipore", (22.526436, 88.343904));
    ("ZWL008828", ZWL008828, "Kolkata Elgin", (22.543063, 88.356096));
    ("ZWL008426", ZWL008426, "Kolkata Howrah", (22.604678, 88.342218));
    ("ZWL007323", ZWL007323, "Kolkata New Alipore", (22.506135, 88.332770));
    ("ZWL006266", ZWL006266, "Kolkata Barasat", (22.735727, 88.490498));
    ("ZWL008393", ZWL008393, "Kolkata New Town (kolkata)", (22.588818, 88.457964));
    ("ZWL007966", ZWL007966, "Kolkata Uttarpara", (22.692420, 88.342900));
    ("ZWL001039", ZWL001039, "Kolkata Santoshpur", (22.502107, 88.388897));
    ("ZWL003882", ZWL003882, "Kolkata Liluah", (22.635342, 88.343314));
    ("ZWL004925", ZWL004925, "Kolkata Rajarhat", (22.621505, 88.446915));
    ("ZWL005244", ZWL005244, "Kolkata Park Street area", (22.552632, 88.362888));
    ("ZWL003687", ZWL003687, "Kolkata Baghajatin Colony", (22.481243, 88.374932));
    ("ZWL008120", ZWL008120, "Kolkata Shrirampur", (22.735733, 88.344597));
    ("ZWL007514", ZWL007514, "Kolkata Chhota Jagulia", (22.759573, 88.747215));
    ("ZWL003935", ZWL003935, "Kolkata Dum Dum", (22.656310, 88.423694));
    ("ZWL002931", ZWL002931, "Kolkata Kestopur", (22.597524, 88.431114));
    ("ZWL006521", ZWL006521, "Kolkata Sodepur", (22.701254, 88.383329));
    ("ZWL005558", ZWL005558, "Kolkata Nimta", (22.670256, 88.413503));
    ("ZWL005174", ZWL005174, "Kolkata Shapoorji", (22.567623, 88.497427));
    ("ZWL002488", ZWL002488, "Kolkata Barabazar Market", (22.574220, 88.366682));
    ("ZWL007934", ZWL007934, "Kolkata Salt Lake 1", (22.573829, 88.412814));
    ("ZWL001584", ZWL001584, "Kolkata Tangra", (22.555059, 88.388175));
    ("ZWL005429", ZWL005429, "Kolkata Gariahat", (22.514492, 88.362000));
    ("ZWL005121", ZWL005121, "Kolkata Santragachi", (22.621044, 88.283605));
    ("ZWL007830", ZWL007830, "Kolkata Garia", (22.469249, 88.382261));
    ("ZWL008991", ZWL008991, "Kolkata Chinsurah, Kolkata", (22.933551, 88.398649));
    ("ZWL009194", ZWL009194, "Kolkata Kankurgachi", (22.573467, 88.391666));
    ("ZWL004722", ZWL004722, "Kolkata Kasba", (22.521693, 88.389844));
    ("ZWL006536", ZWL006536, "Mumbai Mira Road East", (19.278130, 72.874722));
    ("ZWL004934", ZWL004934, "Mumbai Kandivali West", (19.212156, 72.828850));
    ("ZWL004821", ZWL004821, "Mumbai Boisar, Mumbai", (19.804797, 72.745805));
    ("ZWL007249", ZWL007249, "Mumbai Bhiwandi", (19.251790, 73.087318));
    ("ZWL001164", ZWL001164, "Mumbai Panvel", (18.998845, 73.108933));
    ("ZWL009537", ZWL009537, "Mumbai Kopar Khairane (Navi Mumbai)", (19.113909, 73.012598));
    ("ZWL007275", ZWL007275, "Mumbai Vashi", (19.067662, 73.004987));
    ("ZWL002556", ZWL002556, "Mumbai Titwala, Mumbai", (19.292555, 73.205899));
    ("ZWL006937", ZWL006937, "Mumbai Kandivali East", (19.200730, 72.865461));
    ("ZWL008636", ZWL008636, "Mumbai Nalasopara", (19.424433, 72.817134));
    ("ZWL003995", ZWL003995, "Mumbai Lower Parel", (19.005114, 72.820143));
    ("ZWL006938", ZWL006938, "Mumbai Goregaon East", (19.163093, 72.872277));
    ("ZWL008550", ZWL008550, "Mumbai Santacruz East", (19.076131, 72.858715));
    ("ZWL004494", ZWL004494, "Mumbai Ghatkopar East, Mumbai", (19.080794, 72.922280));
    ("ZWL003708", ZWL003708, "Mumbai Palava", (19.168067, 73.074612));
    ("ZWL008711", ZWL008711, "Mumbai Malad West", (19.187022, 72.827076));
    ("ZWL005991", ZWL005991, "Mumbai Borivali East", (19.241003, 72.865970));
    ("ZWL002865", ZWL002865, "Mumbai Kalyan,West,Mumbai", (19.255127, 73.128176));
    ("ZWL004749", ZWL004749, "Mumbai Santacruz West", (19.082610, 72.835608));
    ("ZWL001274", ZWL001274, "Mumbai Badlapur, Mumbai", (19.174156, 73.225767));
    ("ZWL007699", ZWL007699, "Mumbai Ambernath", (19.199776, 73.181176));
    ("ZWL001764", ZWL001764, "Mumbai Marine lines", (18.952178, 72.825198));
    ("ZWL002921", ZWL002921, "Mumbai Mulund West", (19.170765, 72.942239));
    ("ZWL001410", ZWL001410, "Mumbai Airoli", (19.170339, 72.995114));
    ("ZWL009757", ZWL009757, "Mumbai Kharghar (Navi Mumbai)", (19.069735, 73.055883));
    ("ZWL002059", ZWL002059, "Mumbai Ulhasnagar (Mumbai)", (19.224136, 73.169340));
    ("ZWL001058", ZWL001058, "Mumbai Ghatkopar West, Mumbai", (19.096740, 72.904562));
    ("ZWL006995", ZWL006995, "Mumbai Bandra West", (19.068857, 72.833000));
    ("ZWL004692", ZWL004692, "Mumbai Dadar West", (19.021199, 72.835378));
    ("ZWL007667", ZWL007667, "Mumbai Andheri West", (19.137106, 72.834828));
    ("ZWL009338", ZWL009338, "Mumbai Vile Parle West", (19.109560, 72.832194));
    ("ZWL009167", ZWL009167, "Mumbai Byculla", (18.974283, 72.833712));
    ("ZWL006032", ZWL006032, "Mumbai Vasai", (19.364358, 72.836612));
    ("ZWL007397", ZWL007397, "Mumbai Kalyan,East (Mumbai)", (19.221503, 73.138111));
    ("ZWL009360", ZWL009360, "Mumbai Ulwe, Mumbai", (18.974217, 73.024914));
    ("ZWL002205", ZWL002205, "Mumbai Fort", (18.940613, 72.834235));
    ("ZWL008975", ZWL008975, "Mumbai Andheri East", (19.108639, 72.874437));
    ("ZWL009252", ZWL009252, "Mumbai Chembur", (19.049840, 72.907847));
    ("ZWL002558", ZWL002558, "Mumbai Palghar, Mumbai", (19.700631, 72.763031));
    ("ZWL002742", ZWL002742, "Mumbai Mumbra, Mumbai", (19.172636, 73.023715));
    ("ZWL004971", ZWL004971, "Mumbai Mulund East", (19.154253, 72.962493));
    ("ZWL008348", ZWL008348, "Mumbai Shirdhon, Mumbai", (19.195881, 73.128886));
    ("ZWL001344", ZWL001344, "Mumbai Virar", (19.461784, 72.799199));
    ("ZWL007606", ZWL007606, "Mumbai Tardeo", (18.961317, 72.805513));
    ("ZWL005697", ZWL005697, "Mumbai Bandra East", (19.064138, 72.852737));
    ("ZWL005442", ZWL005442, "Mumbai Thane West", (19.227309, 72.972736));
    ("ZWL004189", ZWL004189, "Mumbai Kurla West", (19.079972, 72.881537));
    ("ZWL005000", ZWL005000, "Mumbai Goregaon West", (19.162722, 72.841610));
    ("ZWL002056", ZWL002056, "Mumbai Hiranandani Estate", (19.267411, 72.967074));
    ("ZWL009826", ZWL009826, "Mumbai Dombivli", (19.210229, 73.102272));
    ("ZWL007889", ZWL007889, "Mumbai Bhandup West", (19.151889, 72.934553));
    ("ZWL008977", ZWL008977, "Mumbai Kamothe", (19.023359, 73.091814));
    ("ZWL006622", ZWL006622, "Mumbai Powai, Mumbai", (19.115248, 72.908952));
    ("ZWL002074", ZWL002074, "Mumbai Naupada", (19.188451, 72.968001));
    ("ZWL008874", ZWL008874, "Mumbai Bhayandar West", (19.257869, 72.804743));
    ("ZWL001334", ZWL001334, "Mumbai Vileparle East", (19.098430, 72.864947));
    ("ZWL008378", ZWL008378, "Mumbai Sion", (19.045271, 72.864631));
    ("ZWL007690", ZWL007690, "Mumbai Nerul (Navi Mumbai)", (19.031072, 73.026647));
    ("ZWL007706", ZWL007706, "Mumbai Dahisar West", (19.255014, 72.848267));
    ("ZWL005320", ZWL005320, "Mumbai Colaba", (18.919405, 72.824376));
    ("ZWL006544", ZWL006544, "Mumbai Mahim", (19.039256, 72.843913));
    ("ZWL002686", ZWL002686, "Mumbai Wadala", (19.017538, 72.867138));
    ("ZWL001089", ZWL001089, "Mumbai Borivali West", (19.229714, 72.839710));
    ("ZWL008360", ZWL008360, "Mumbai Palava Lakeshore,Mumbai", (19.165716, 73.105733));
    ("ZWL003467", ZWL003467, "Bengaluru Banashankari", (12.936787, 77.556079));
    ("ZWL004900", ZWL004900, "Bengaluru Rajarajeshwari Nagar", (12.918637, 77.505467));
    ("ZWL005530", ZWL005530, "Bengaluru JP Nagar", (12.893441, 77.560436));
    ("ZWL007643", ZWL007643, "Bengaluru Mahadevapura", (12.985322, 77.687578));
    ("ZWL003159", ZWL003159, "Bengaluru Jalahalli", (13.031518, 77.530986));
    ("ZWL002736", ZWL002736, "Bengaluru RT Nagar", (13.021267, 77.601234));
    ("ZWL006369", ZWL006369, "Bengaluru KR Puram", (13.016987, 77.706819));
    ("ZWL006274", ZWL006274, "Bengaluru Electronic City", (12.833101, 77.673182));
    ("ZWL005375", ZWL005375, "Bengaluru Vijayanagar", (12.973219, 77.519303));
    ("ZWL008600", ZWL008600, "Bengaluru Marathahalli", (12.955103, 77.696507));
    ("ZWL002292", ZWL002292, "Bengaluru Sarjapur road", (12.900225, 77.697451));
    ("ZWL004341", ZWL004341, "Bengaluru Brookefields", (12.967420, 77.717851));
    ("ZWL007633", ZWL007633, "Bengaluru Whitefield", (12.975224, 77.740422));
    ("ZWL009212", ZWL009212, "Bengaluru Nagavara", (13.048370, 77.625534));
    ("ZWL007628", ZWL007628, "Bengaluru New BEL Road", (13.040495, 77.569420));
    ("ZWL001156", ZWL001156, "Bengaluru Koramangala", (12.933756, 77.625825));
    ("ZWL004924", ZWL004924, "Bengaluru Bannerghatta Road, Bangalore", (12.891397, 77.608176));
    ("ZWL009229", ZWL009229, "Bengaluru Aavalahalli", (13.034488, 77.712241));
    ("ZWL005576", ZWL005576, "Bengaluru BIAL Airport Road", (13.178996, 77.630005));
    ("ZWL006658", ZWL006658, "Bengaluru Yelahanka", (13.111809, 77.589276));
    ("ZWL002882", ZWL002882, "Bengaluru Kadugodi", (13.007511, 77.763209));
    ("ZWL006631", ZWL006631, "Bengaluru Kammanahalli", (13.016050, 77.661735));
    ("ZWL001196", ZWL001196, "Bengaluru HSR Layout", (12.908482, 77.641773));
    ("ZWL006600", ZWL006600, "Bengaluru BTM Layout", (12.916931, 77.608897));
    ("ZWL006854", ZWL006854, "Bengaluru Varthur", (12.936055, 77.723415));
    ("ZWL001273", ZWL001273, "Bengaluru Indiranagar", (12.952636, 77.653059));
    ("ZWL008105", ZWL008105, "Bengaluru Jayanagar", (12.944441, 77.581003));
    ("ZWL005206", ZWL005206, "Bengaluru Sahakaranagar", (13.059918, 77.591344));
    ("ZWL008797", ZWL008797, "Bengaluru Devanahalli, Bangalore", (13.258381, 77.716183));
    ("ZWL001962", ZWL001962, "Bengaluru MG Road", (12.982689, 77.608075));
    ("ZWL006844", ZWL006844, "Bengaluru Rajajinagar", (12.993217, 77.557903));
    ("ZWL004164", ZWL004164, "Bengaluru Bellandur", (12.936225, 77.665059));
    ("ZWL007698", ZWL007698, "Pune NIGDI(Pune)", (18.646511, 73.775411));
    ("ZWL005773", ZWL005773, "Pune Bibvewadi (Pune)", (18.492332, 73.861939));
    ("ZWL009577", ZWL009577, "Pune Nanded-Nahre", (18.453179, 73.811077));
    ("ZWL002253", ZWL002253, "Pune Bhosari (Pune)", (18.643893, 73.858653));
    ("ZWL003498", ZWL003498, "Pune Camp Area", (18.513693, 73.877293));
    ("ZWL004311", ZWL004311, "Pune Magarpatta", (18.519482, 73.934360));
    ("ZWL007801", ZWL007801, "Pune Pimpri", (18.625381, 73.791194));
    ("ZWL008134", ZWL008134, "Pune Yerwada", (18.544659, 73.869499));
    ("ZWL009625", ZWL009625, "Pune Kalyani Nagar (Pune)", (18.546538, 73.906594));
    ("ZWL001627", ZWL001627, "Pune Sus, Pune", (18.564293, 73.753879));
    ("ZWL006236", ZWL006236, "Pune Bavdhan", (18.514730, 73.777922));
    ("ZWL006743", ZWL006743, "Pune Viman nagar", (18.564998, 73.911551));
    ("ZWL004927", ZWL004927, "Pune Aundh (Pune)", (18.559795, 73.807123));
    ("ZWL008370", ZWL008370, "Pune Katraj (Pune)", (18.451394, 73.847642));
    ("ZWL004523", ZWL004523, "Pune", (18.600897, 73.798441));
    ("ZWL001778", ZWL001778, "Pune Dhanori", (18.588936, 73.907711));
    ("ZWL003386", ZWL003386, "Pune Dehu Road", (18.695874, 73.740365));
    ("ZWL009157", ZWL009157, "Pune Koregaon Park (Pune)", (18.535326, 73.883976));
    ("ZWL004962", ZWL004962, "Pune Hinjewadi - Phase 2", (18.585004, 73.706319));
    ("ZWL005340", ZWL005340, "Pune", (18.599555, 73.774652));
    ("ZWL009671", ZWL009671, "Pune Manas Lake, Pune", (18.491382, 73.748953));
    ("ZWL008921", ZWL008921, "Pune Sadashiv Peth", (18.511357, 73.856996));
    ("ZWL008940", ZWL008940, "Pune Ghorpadi (Pune)", (18.519767, 73.897269));
    ("ZWL009874", ZWL009874, "Pune Pashan (Pune)", (18.527587, 73.789459));
    ("ZWL006660", ZWL006660, "Pune SP Infocity, Pune", (18.487813, 73.947187));
    ("ZWL003813", ZWL003813, "Pune Manjri,Pune", (18.509232, 73.978364));
    ("ZWL007471", ZWL007471, "Pune Khadki, Pune", (18.563289, 73.835023));
    ("ZWL009602", ZWL009602, "Pune Kothrud (Pune)", (18.504984, 73.811587));
    ("ZWL002208", ZWL002208, "Pune Sinhagad Road(Pune)", (18.477264, 73.826437));
    ("ZWL007895", ZWL007895, "Pune New Sangvi, Pune", (18.578959, 73.823437));
    ("ZWL003099", ZWL003099, "Pune Nimgaon, Pune", (18.794589, 73.910016));
    ("ZWL004575", ZWL004575, "Pune Wagholi", (18.579310, 73.972122));
    ("ZWL004513", ZWL004513, "Pune Keshavnagar, Pune", (18.537273, 73.944521));
    ("ZWL008983", ZWL008983, "Pune Baner (Pune)", (18.567954, 73.783475));
    ("ZWL005520", ZWL005520, "Pune Warje (Pune)", (18.472703, 73.788400));
    ("ZWL003300", ZWL003300, "Pune Mundwa, Pune", (18.535220, 73.918549));
    ("ZWL001963", ZWL001963, "Pune Chakan", (18.752436, 73.837484));
    ("ZWL002422", ZWL002422, "Pune Shivaji Nagar (Pune)", (18.526724, 73.841658));
    ("ZWL005088", ZWL005088, "Pune Yewalewadi, Pune", (18.439804, 73.902433));
    ("ZWL003014", ZWL003014, "Pune Bopkhel, Pune", (18.586306, 73.859602));
    ("ZWL001472", ZWL001472, "Pune Kharadi", (18.552390, 73.941901));
    ("ZWL004339", ZWL004339, "Pune Talegaon Dabhade", (18.738724, 73.675701));
    ("ZWL003817", ZWL003817, "Pune Hinjewadi - Phase 1", (18.596583, 73.733312));
    ("ZWL007925", ZWL007925, "Pune Wanowrie-Kondhwa", (18.477085, 73.900966));
    ("ZWL003370", ZWL003370, "Hyderabad Nagole", (17.359969, 78.565724));
    ("ZWL002088", ZWL002088, "Hyderabad Attapur", (17.380875, 78.415717));
    ("ZWL006702", ZWL006702, "Hyderabad Peerzadiguda", (17.411644, 78.578220));
    ("ZWL008776", ZWL008776, "Hyderabad Begumpet", (17.442659, 78.482009));
    ("ZWL003918", ZWL003918, "Hyderabad Suraram, Hyderabad", (17.559752, 78.437467));
    ("ZWL004079", ZWL004079, "Hyderabad Banjara Hills", (17.419238, 78.438474));
    ("ZWL008309", ZWL008309, "Hyderabad Alwal", (17.508852, 78.507402));
    ("ZWL002433", ZWL002433, "Hyderabad Sainikpuri", (17.489079, 78.558910));
    ("ZWL007390", ZWL007390, "Hyderabad Saroor Nagar", (17.348264, 78.530476));
    ("ZWL004767", ZWL004767, "Hyderabad Karkhana", (17.467142, 78.496696));
    ("ZWL006016", ZWL006016, "Hyderabad Kompally", (17.533309, 78.489965));
    ("ZWL003283", ZWL003283, "Hyderabad Himayatnagar", (17.402602, 78.487229));
    ("ZWL007311", ZWL007311, "Hyderabad Medchal Road", (17.637191, 78.480392));
    ("ZWL005919", ZWL005919, "Hyderabad Kukatpally", (17.495894, 78.416259));
    ("ZWL001822", ZWL001822, "Hyderabad Amberpet", (17.407658, 78.521703));
    ("ZWL008208", ZWL008208, "Hyderabad Jeedimetla", (17.495621, 78.450281));
    ("ZWL001362", ZWL001362, "Hyderabad Gachibowli", (17.452114, 78.351486));
    ("ZWL002162", ZWL002162, "Hyderabad LB Nagar", (17.345727, 78.557062));
    ("ZWL009712", ZWL009712, "Hyderabad Dilsukhnagar", (17.370431, 78.536184));
    ("ZWL005963", ZWL005963, "Hyderabad Masab Tank", (17.398217, 78.462258));
    ("ZWL008297", ZWL008297, "Hyderabad Bachupally", (17.544887, 78.359975));
    ("ZWL005424", ZWL005424, "Hyderabad Manikonda", (17.405930, 78.388380));
    ("ZWL008585", ZWL008585, "Hyderabad Shamshabad", (17.257432, 78.387920));
    ("ZWL008599", ZWL008599, "Hyderabad Miyapur", (17.500034, 78.342670));
    ("ZWL006535", ZWL006535, "Hyderabad Hayath Nagar, Hyderabad", (17.326620, 78.609288));
    ("ZWL006545", ZWL006545, "Hyderabad Sangareddy, Hyderabad", (17.598038, 78.091346));
    ("ZWL009719", ZWL009719, "Hyderabad JNTU", (17.485166, 78.389775));
    ("ZWL008890", ZWL008890, "Hyderabad Serilingampally", (17.485245, 78.313916));
    ("ZWL007119", ZWL007119, "Hyderabad Nizampet", (17.509398, 78.390364));
    ("ZWL004747", ZWL004747, "Hyderabad Q City, Hyderabad", (17.421788, 78.318868));
    ("ZWL004802", ZWL004802, "Hyderabad Madhapur", (17.441185, 78.398416));
    ("ZWL004665", ZWL004665, "Hyderabad Narayanguda", (17.392120, 78.494443));
    ("ZWL005999", ZWL005999, "Hyderabad ECIL", (17.460327, 78.567924));
    ("ZWL003360", ZWL003360, "Hyderabad Toli Chowki", (17.398121, 78.421238));
    ("ZWL007187", ZWL007187, "Hyderabad Kondapur", (17.472485, 78.367854));
    ("ZWL007344", ZWL007344, "Hyderabad Charminar", (17.375037, 78.454499));
    ("ZWL008438", ZWL008438, "Hyderabad Sivarampalli", (17.345995, 78.439283));
    ("ZWL005494", ZWL005494, "Hyderabad Tarnaka", (17.431829, 78.550940));
    ("ZWL009519", ZWL009519, "Hyderabad Moosapet", (17.453292, 78.421126));
    ("ZWL001687", ZWL001687, "Hyderabad Patancheru, Hyderabad", (17.542613, 78.276529));
    ("ZWL005569", ZWL005569, "Hyderabad Vanasthali Puram", (17.318241, 78.544989));
    ("ZWL003027", ZWL003027, "Hyderabad Ameerpet", (17.434737, 78.446618));
    ("ZWL001337", ZWL001337, "Hyderabad Uppal", (17.403149, 78.554106));
    ("ZWL001579", ZWL001579, "Hyderabad Malakpet", (17.371621, 78.502672));
    ("ZWL006699", ZWL006699, "Hyderabad Hafiz Baba Nagar", (17.338717, 78.497590));
    ("ZWL008512", ZWL008512, "Hyderabad Mokila, Hyderabad", (17.436228, 78.190072));
    ("ZWL006789", ZWL006789, "Chennai Potheri", (12.799983, 80.029865));
    ("ZWL004297", ZWL004297, "Chennai Pallavaram", (12.973055, 80.151271));
    ("ZWL005190", ZWL005190, "Chennai Nungambakkam", (13.060471, 80.255887));
    ("ZWL003967", ZWL003967, "Chennai Anna Nagar, Chennai", (13.086884, 80.206602));
    ("ZWL008996", ZWL008996, "Chennai Perambur", (13.121741, 80.225058));
    ("ZWL005857", ZWL005857, "Chennai Mogappair, Chennai", (13.080365, 80.175724));
    ("ZWL006232", ZWL006232, "Chennai Royapuram", (13.136635, 80.289535));
    ("ZWL008548", ZWL008548, "Chennai Mugalivakkam", (13.014886, 80.152455));
    ("ZWL006053", ZWL006053, "Chennai Porur", (13.048069, 80.158163));
    ("ZWL001398", ZWL001398, "Chennai Redhills", (13.191443, 80.181225));
    ("ZWL001701", ZWL001701, "Chennai Tambaram", (12.934834, 80.101824));
    ("ZWL008876", ZWL008876, "Chennai Avadi", (13.125301, 80.069776));
    ("ZWL003387", ZWL003387, "Chennai Kilpauk", (13.080772, 80.248018));
    ("ZWL007059", ZWL007059, "Chennai Ashok Nagar (CHENNAI)", (13.022680, 80.200286));
    ("ZWL006520", ZWL006520, "Chennai Adyar", (12.993947, 80.247174));
    ("ZWL001210", ZWL001210, "Chennai Alwarpet", (13.032366, 80.257625));
    ("ZWL007171", ZWL007171, "Chennai Selaiyur", (12.916570, 80.134348));
    ("ZWL006329", ZWL006329, "Chennai Thandalam, Chennai", (12.863785, 79.947886));
    ("ZWL004233", ZWL004233, "Chennai Sholinganallur", (12.921608, 80.233727));
    ("ZWL007209", ZWL007209, "Chennai Ambattur", (13.117566, 80.146667));
    ("ZWL003452", ZWL003452, "Chennai Medavakkam", (12.931197, 80.182327));
    ("ZWL007176", ZWL007176, "Chennai Poonamallee", (13.052763, 80.090763));
    ("ZWL009897", ZWL009897, "Chennai Minjur, Chennai", (13.282298, 80.266616));
    ("ZWL004882", ZWL004882, "Chennai Urapakkam", (12.878957, 80.070307));
    ("ZWL006156", ZWL006156, "Chennai Velachery", (12.989300, 80.199988));
    ("ZWL001141", ZWL001141, "Chennai Navallur", (12.841923, 80.209025));
    ("ZWL004431", ZWL004431, "Chennai Vadapalani", (13.065160, 80.207917));
    ("ZWL001516", ZWL001516, "Chennai T Nagar", (13.026256, 80.228120));
    ("ZWL003425", ZWL003425, "Lucknow Hazratganj", (26.844819, 80.940833));
    ("ZWL006490", ZWL006490, "Lucknow Aminabad", (26.856770, 80.925103));
    ("ZWL009091", ZWL009091, "Lucknow Chowk", (26.873686, 80.894502));
    ("ZWL003030", ZWL003030, "Lucknow Telibagh, Lucknow", (26.776664, 80.936755));
    ("ZWL004978", ZWL004978, "Lucknow Husainganj", (26.836970, 80.926110));
    ("ZWL009177", ZWL009177, "Lucknow Jankipuram", (26.926427, 80.936884));
    ("ZWL004436", ZWL004436, "Lucknow Arjunganj", (26.794554, 80.999112));
    ("ZWL005470", ZWL005470, "Lucknow Chinhat, Lucknow", (26.878323, 81.038299));
    ("ZWL001500", ZWL001500, "Lucknow Mahanagar", (26.886387, 80.960641));
    ("ZWL003371", ZWL003371, "Lucknow Ashiyana", (26.790510, 80.913855));
    ("ZWL003635", ZWL003635, "Lucknow Indira Nagar, Lucknow", (26.883684, 80.990290));
    ("ZWL003320", ZWL003320, "Lucknow Vasant Kunj, Lucknow", (26.877121, 80.878447));
    ("ZWL009682", ZWL009682, "Lucknow Rajajipuram", (26.842055, 80.873868));
    ("ZWL002273", ZWL002273, "Lucknow Gomti Nagar", (26.850582, 80.999875));
    ("ZWL002331", ZWL002331, "Lucknow Alambagh", (26.810769, 80.904864));
    ("ZWL007731", ZWL007731, "Lucknow Aliganj, Lucknow", (26.890082, 80.942007));
    ("ZWL007011", ZWL007011, "Lucknow Kalyanpur", (26.907206, 80.974143));
    ("ZWL003768", ZWL003768, "Kochi Eroor", (9.929826, 76.332369));
    ("ZWL005555", ZWL005555, "Kochi Kakkanad", (10.014153, 76.358626));
    ("ZWL002986", ZWL002986, "Kochi Nedumbassery,Kochi", (10.178008, 76.386636));
    ("ZWL004273", ZWL004273, "Kochi North Paravur, Kochi", (10.162441, 76.216317));
    ("ZWL005425", ZWL005425, "Kochi Aluva, Kochi", (10.106615, 76.348843));
    ("ZWL004691", ZWL004691, "Kochi Ambalamugal", (9.980241, 76.397987));
    ("ZWL001981", ZWL001981, "Kochi Kalamassery", (10.046034, 76.308117));
    ("ZWL003786", ZWL003786, "Kochi Kaloor", (10.000143, 76.298744));
    ("ZWL009487", ZWL009487, "Kochi Perumbavoor,Kochi", (10.117563, 76.460320));
    ("ZWL007216", ZWL007216, "Kochi Thiruvankulam", (9.932264, 76.385740));
    ("ZWL002327", ZWL002327, "Kochi Vypin, Kochi", (10.019731, 76.245595));
    ("ZWL006873", ZWL006873, "Kochi Ernakulam", (9.969514, 76.288288));
    ("ZWL004591", ZWL004591, "Kochi Fort Kochi", (9.931564, 76.268065));
    ("ZWL005082", ZWL005082, "Jaipur Mansarovar-2", (26.844258, 75.768570));
    ("ZWL003863", ZWL003863, "Jaipur Tonk road 2", (26.853567, 75.794945));
    ("ZWL009286", ZWL009286, "Jaipur Jagatpura", (26.825942, 75.851086));
    ("ZWL009458", ZWL009458, "Jaipur Shyam Nagar", (26.887617, 75.756174));
    ("ZWL003704", ZWL003704, "Jaipur C Scheme", (26.916823, 75.801190));
    ("ZWL003606", ZWL003606, "Jaipur Lal Kothi", (26.893689, 75.797800));
    ("ZWL009569", ZWL009569, "Jaipur Pink City", (26.929588, 75.823855));
    ("ZWL001750", ZWL001750, "Jaipur Vaishali Nagar", (26.909885, 75.739394));
    ("ZWL002751", ZWL002751, "Jaipur Malviya Nagar", (26.854191, 75.810798));
    ("ZWL005080", ZWL005080, "Jaipur Sodala", (26.904751, 75.777608));
    ("ZWL008680", ZWL008680, "Jaipur Vidhyadhar Nagar", (26.963051, 75.770166));
    ("ZWL008249", ZWL008249, "Jaipur Raja Park", (26.902497, 75.826544));
    ("ZWL008915", ZWL008915, "Jaipur Shastri Nagar", (26.936514, 75.797054));
    ("ZWL006372", ZWL006372, "Jaipur Pratap Nagar", (26.798396, 75.815353));
    ("ZWL003133", ZWL003133, "Ahmedabad Paldi", (22.994998, 72.557474));
    ("ZWL002302", ZWL002302, "Ahmedabad Shahibag", (23.058607, 72.592212));
    ("ZWL003747", ZWL003747, "Ahmedabad Navrangpura", (23.038426, 72.558241));
    ("ZWL002503", ZWL002503, "Ahmedabad Chandkheda", (23.117204, 72.607123));
    ("ZWL005979", ZWL005979, "Ahmedabad Science-City Sola", (23.087361, 72.510289));
    ("ZWL005987", ZWL005987, "Ahmedabad Sector 16, Gandhinagar", (23.216612, 72.652543));
    ("ZWL001959", ZWL001959, "Ahmedabad Vastrapur", (23.042513, 72.524312));
    ("ZWL007404", ZWL007404, "Ahmedabad Prahlad Nagar", (22.998074, 72.515955));
    ("ZWL001898", ZWL001898, "Ahmedabad Nikol", (23.077071, 72.637566));
    ("ZWL002250", ZWL002250, "Ahmedabad Infocity, Gandhinagar", (23.177762, 72.636221));
    ("ZWL004415", ZWL004415, "Ahmedabad Naranpura", (23.072018, 72.573161));
    ("ZWL006288", ZWL006288, "Ahmedabad Bopal", (23.011294, 72.464041));
    ("ZWL009182", ZWL009182, "Ahmedabad Maninagar", (22.985307, 72.610322));
    ("ZWL009990", ZWL009990, "Ahmedabad Bodakdev", (23.055398, 72.491724));
    ("ZWL003455", ZWL003455, "Ahmedabad Gota", (23.131231, 72.543606));
    ("ZWL007561", ZWL007561, "Chandigarh Sector 15 (Chandigarh)", (30.764466, 76.774815));
    ("ZWL006687", ZWL006687, "Chandigarh Sector 8 (Chandigarh)", (30.749265, 76.801584));
    ("ZWL001934", ZWL001934, "Chandigarh Manimajra (Chandigarh)", (30.722848, 76.835395));
    ("ZWL003936", ZWL003936, "Chandigarh Industrial Area Phase I (Chandigarh)", (30.701381, 76.808231));
    ("ZWL004716", ZWL004716, "Chandigarh Sector 59 (Chandigarh)", (30.725954, 76.716657));
    ("ZWL002303", ZWL002303, "Chandigarh Sector 20, Panchkula", (30.667002, 76.860170));
    ("ZWL009521", ZWL009521, "9 nd Panchkula", (30.694507, 76.849604));
    ("ZWL006817", ZWL006817, "Chandigarh Sector 28 (Chandigarh)", (30.724245, 76.808833));
    ("ZWL003496", ZWL003496, "Chandigarh Phase 10 Mohali", (30.692094, 76.733846));
    ("ZWL009894", ZWL009894, "Chandigarh Gillco, Chandigarh", (30.736526, 76.654293));
    ("ZWL004101", ZWL004101, "Chandigarh Sector 46 (Chandigarh)", (30.705920, 76.762310));
    ("ZWL003262", ZWL003262, "Chandigarh Sector 70 (Chandigarh)", (30.709870, 76.692050));
    ("ZWL009430", ZWL009430, "Chandigarh VIP Road, Zirakpur", (30.656048, 76.823298));
    ("ZWL004196", ZWL004196, "Chandigarh VR Mall", (30.730066, 76.684242));
    ("ZWL003093", ZWL003093, "Chandigarh Sector 35 (Chandigarh)", (30.727290, 76.756632));
    ("ZWL009406", ZWL009406, "Chandigarh Sector 22 (Chandigarh)", (30.735307, 76.774912));
    ("ZWL002150", ZWL002150, "Goa Verna, Goa", (15.380640, 73.909304));
    ("ZWL008519", ZWL008519, "Goa Mapusa, Goa", (15.599556, 73.791137));
    ("ZWL004452", ZWL004452, "Goa Calangute, Goa", (15.532704, 73.762608));
    ("ZWL006556", ZWL006556, "Goa Majorda, Goa", (15.255895, 73.937616));
    ("ZWL002137", ZWL002137, "Goa Upper panaji, Goa", (15.460913, 73.835391));
    ("ZWL005093", ZWL005093, "Goa Ponda, Goa", (15.398953, 74.002377));
    ("ZWL002142", ZWL002142, "Goa Margao, Goa", (15.231363, 73.997689));
    ("ZWL004621", ZWL004621, "Goa Vasco, Goa", (15.387592, 73.829979));
    ("ZWL006403", ZWL006403, "Goa Morjim, Goa", (15.653757, 73.747022));
    ("ZWL005568", ZWL005568, "Goa Porvorim, Goa", (15.535007, 73.827229));
    ("ZWL009021", ZWL009021, "Ludhiana Sector 32, Ludhiana", (30.895881, 75.906050));
    ("ZWL006208", ZWL006208, "Ludhiana Civil Lines, Ludhiana", (30.916548, 75.818266));
    ("ZWL006788", ZWL006788, "Ludhiana Sarabha Nagar, Ludhiana", (30.891869, 75.834901));
    ("ZWL005256", ZWL005256, "Ludhiana BRS Nagar, Ludhiana", (30.897774, 75.787544));
    ("ZWL001163", ZWL001163, "Ludhiana Model Town, Ludhiana", (30.881060, 75.875309));
    ("ZWL003304", ZWL003304, "Ludhiana Ganesh Nagar, Ludhiana", (30.917540, 75.879759));
    ("ZWL003119", ZWL003119, "Ludhiana Dugri,Ludhiana", (30.856247, 75.843129));
    ("ZWL006981", ZWL006981, "Guwahati Pathar Quarry, Guwahati", (26.159879, 91.827651));
    ("ZWL006537", ZWL006537, "Guwahati Basistha-Lokhra, Guwahati", (26.116888, 91.777488));
    ("ZWL009407", ZWL009407, "Guwahati North Guwahati, Guwahati", (26.196471, 91.684788));
    ("ZWL004763", ZWL004763, "Guwahati Dharapur, Guwahati", (26.134004, 91.623895));
    ("ZWL007095", ZWL007095, "Guwahati Lal Ganesh - Kahilipara, Guwahati", (26.145102, 91.752656));
    ("ZWL002105", ZWL002105, "Guwahati Paltan-Bazar, Guwahati", (26.183358, 91.752480));
    ("ZWL003362", ZWL003362, "Guwahati Azara, Guwahati", (26.110966, 91.602115));
    ("ZWL002491", ZWL002491, "Guwahati Changsari, Guwahati", (26.262376, 91.694879));
    ("ZWL005319", ZWL005319, "Guwahati Maligaon - Jalukbari, Guwahati", (26.161285, 91.689576));
    ("ZWL005708", ZWL005708, "Guwahati Zoo Tiniali - Christian basti", (26.177033, 91.779799));
    ("ZWL001780", ZWL001780, "Amritsar Himatpura, Amritsar", (31.608999, 74.863051));
    ("ZWL008281", ZWL008281, "Amritsar Rasulpur, Amritsar", (31.619590, 74.915049));
    ("ZWL004590", ZWL004590, "Amritsar Ranjit Avenue, Amritsar", (31.666773, 74.850200));
    ("ZWL002073", ZWL002073, "Amritsar White Avenue, Amritsar", (31.657377, 74.889594));
    ("ZWL005826", ZWL005826, "Amritsar Chheharta, Amritsar", (31.633049, 74.817702));
    ("ZWL007456", ZWL007456, "Amritsar Hall Bazar, Amritsar", (31.632158, 74.866838));
    ("ZWL008755", ZWL008755, "Bhopal Ashoka Garden, Bhopal", (23.260793, 77.421525));
    ("ZWL009428", ZWL009428, "Bhopal Shahpura,Bhopal", (23.176545, 77.418255));
    ("ZWL006900", ZWL006900, "Bhopal Airport Area, Bhopal", (23.290449, 77.337426));
    ("ZWL002615", ZWL002615, "Bhopal TT Nagar, Bhopal", (23.223600, 77.392134));
    ("ZWL003463", ZWL003463, "Bhopal BHEL, Bhopal", (23.267602, 77.474332));
    ("ZWL002872", ZWL002872, "Bhopal MP Nagar,Bhopal", (23.227641, 77.447330));
    ("ZWL005836", ZWL005836, "Bhopal Hoshangabad Road, Bhopal", (23.168118, 77.482657));
    ("ZWL003417", ZWL003417, "s Mall, Bhopal", (23.293081, 77.396385));
    ("ZWL001466", ZWL001466, "Visakhapatnam NAD, Vizag", (17.749616, 83.230677));
    ("ZWL003024", ZWL003024, "Visakhapatnam Gajuwaka", (17.681004, 83.207459));
    ("ZWL004755", ZWL004755, "Visakhapatnam Dwaraka Nagar", (17.739116, 83.324672));
    ("ZWL009959", ZWL009959, "Visakhapatnam Madhurawada", (17.789823, 83.373031));
    ("ZWL007491", ZWL007491, "Bhubaneswar Madhusudan Nagar, Bhubaneswar", (20.278497, 85.824922));
    ("ZWL003084", ZWL003084, "Bhubaneswar Kalinga Nagar, Bhubneshwar", (20.277934, 85.773884));
    ("ZWL003270", ZWL003270, "Bhubaneswar Nayapalli, Bhubneshwar", (20.299879, 85.813518));
    ("ZWL007379", ZWL007379, "Bhubaneswar Sahid Nagar, Bhubaneshwar", (20.289125, 85.856680));
    ("ZWL001823", ZWL001823, "Bhubaneswar Lakshmi Sagar, Bhubneshwar", (20.266492, 85.851166));
    ("ZWL004098", ZWL004098, "Bhubaneswar Khandagiri, Bhubneshwar", (20.246979, 85.766384));
    ("ZWL008906", ZWL008906, "Bhubaneswar Jagmohan Nagar, Bhubaneswar", (20.257002, 85.789371));
    ("ZWL002821", ZWL002821, "Bhubaneswar Kharabela Nagar, Bhubaneswar", (20.271584, 85.840940));
    ("ZWL009572", ZWL009572, "Bhubaneswar Chandrasekharpur, Bhubaneswar", (20.317124, 85.806992));
    ("ZWL005652", ZWL005652, "Bhubaneswar Patia, Bhubneshwar", (20.363954, 85.814321));
    ("ZWL003661", ZWL003661, "Coimbatore Gandhipuram, Coimbatore", (11.019095, 76.968472));
    ("ZWL005742", ZWL005742, "Coimbatore Vadavalli", (11.024820, 76.900648));
    ("ZWL008653", ZWL008653, "Coimbatore RS Puram, Coimbatore", (11.000355, 76.941985));
    ("ZWL002703", ZWL002703, "Coimbatore Racecourse, Coimbatore", (10.999272, 76.975662));
    ("ZWL009668", ZWL009668, "Coimbatore Saibaba Colony, Coimbatore", (11.032946, 76.944253));
    ("ZWL007527", ZWL007527, "Coimbatore Peelamedu, Coimbatore", (11.026179, 77.005899));
    ("ZWL005468", ZWL005468, "Coimbatore Podanur, Coimbatore", (10.977206, 76.981702));
    ("ZWL004408", ZWL004408, "Coimbatore Kunniamuthur, Coimbatore", (10.937589, 76.933705));
    ("ZWL008265", ZWL008265, "Coimbatore Ondipudur, Coimbatore", (10.997670, 77.042986));
    ("ZWL007600", ZWL007600, "Coimbatore Koundampalayam", (11.068353, 76.937962));
    ("ZWL002147", ZWL002147, "Coimbatore Saravanampatty", (11.070812, 76.998053));
    ("ZWL009595", ZWL009595, "Coimbatore Ganapathypudur, Coimbatore", (11.042698, 76.984477));
    ("ZWL001279", ZWL001279, "Coimbatore Sitra, and Singanallur, Coimbatore", (11.042569, 77.054996));
    ("ZWL006449", ZWL006449, "Mangalore South Mangalore", (12.879030, 74.854593));
    ("ZWL009478", ZWL009478, "Mangalore Thokkattu, Mangalore", (12.820460, 74.865765));
    ("ZWL002354", ZWL002354, "Vadodara Waghodia", (22.300216, 73.229259));
    ("ZWL004097", ZWL004097, "Vadodara Fatehgunj", (22.315398, 73.199769));
    ("ZWL009713", ZWL009713, "Vadodara Nizampura", (22.334731, 73.176306));
    ("ZWL008938", ZWL008938, "Vadodara Diwalipura", (22.302691, 73.156120));
    ("ZWL004439", ZWL004439, "Vadodara Akota", (22.298674, 73.178672));
    ("ZWL002446", ZWL002446, "Vadodara Manjalpur, Vadodara", (22.257070, 73.191150));
    ("ZWL008232", ZWL008232, "Vadodara Shubhanpura", (22.321614, 73.160014));
    ("ZWL002475", ZWL002475, "Vadodara Alkapuri", (22.313140, 73.172760));
    ("ZWL005549", ZWL005549, "Nagpur Pratap Nagar", (21.114916, 79.051774));
    ("ZWL001438", ZWL001438, "Nagpur Sadar", (21.171681, 79.072065));
    ("ZWL006432", ZWL006432, "Nagpur Kharabi, Nagpur", (21.144692, 79.136306));
    ("ZWL009782", ZWL009782, "Nagpur Hanuman Nagar", (21.131152, 79.100974));
    ("ZWL008282", ZWL008282, "Nagpur Dharampeth", (21.131547, 79.056027));
    ("ZWL001041", ZWL001041, "Nagpur Manish Nagar", (21.091663, 79.072458));
    ("ZWL007188", ZWL007188, "Nagpur Ayodhya Nagar, Nagpur", (21.103625, 79.104888));
    ("ZWL003633", ZWL003633, "Nagpur Gandhibagh", (21.146868, 79.103994));
    ("ZWL002458", ZWL002458, "Mysore Central Mysore", (12.326689, 76.633539));
    ("ZWL005095", ZWL005095, "Surat Udhna, Surat", (21.166218, 72.850231));
    ("ZWL002155", ZWL002155, "Surat City Light, Surat", (21.159272, 72.791465));
    ("ZWL007951", ZWL007951, "Surat Athwa", (21.168804, 72.803931));
    ("ZWL006000", ZWL006000, "Surat Vesu, Surat", (21.136777, 72.762895));
    ("ZWL008198", ZWL008198, "Surat Adajan, Surat", (21.211151, 72.793110));
    ("ZWL002771", ZWL002771, "Surat Varaccha, Surat", (21.210937, 72.857374));
    ("ZWL005626", ZWL005626, "Surat New Textile Market, Surat", (21.199018, 72.828239));
    ("ZWL005423", ZWL005423, "Surat Katargam, Surat", (21.231792, 72.824230));
    ("ZWL009343", ZWL009343, "Trivandrum Kazhakoottam, Thiruvananthapuram", (8.575170, 76.904888));
    ("ZWL007746", ZWL007746, "Trivandrum Tvm Central", (8.490977, 76.969250));
    ("ZWL002223", ZWL002223, "Trivandrum Nemom, Thiruvananthapuram", (8.430376, 77.027424));
    ("ZWL005308", ZWL005308, "Vijayawada Governorpet, Vijayawada", (16.520989, 80.636340));
    ("ZWL004428", ZWL004428, "Vijayawada Gunadala, Vijayawada", (16.515748, 80.690025));
    ("ZWL002106", ZWL002106, "Vijayawada Gollapudi, Vijayawada", (16.541337, 80.585061));
    ("ZWL005858", ZWL005858, "Vijayawada Auto Nagar, Vijayawada", (16.487591, 80.685560));
    ("ZWL003905", ZWL003905, "Vijayawada Labbipet, Vijayawada", (16.510686, 80.648812));
    ("ZWL009921", ZWL009921, "Jalandhar Shastri Nagar, Jalandhar", (31.323693, 75.583627));
    ("ZWL002344", ZWL002344, "Jalandhar Gurdev Nagar, Jalandhar", (31.347368, 75.566556));
    ("ZWL001077", ZWL001077, "Jalandhar Paragpur, Jalandhar", (31.285004, 75.648457));
    ("ZWL005408", ZWL005408, "Jalandhar Model Town, Jalandhar", (31.299818, 75.582441));
    ("ZWL001624", ZWL001624, "Jalandhar Basti Nau, Jalandhar", (31.327456, 75.549901));
    ("ZWL004713", ZWL004713, "Jalandhar Rama Mandi, Jalandhar", (31.314281, 75.617635));
    ("ZWL007457", ZWL007457, "Jammu Greater Kailash, Jammu", (32.670712, 74.901243));
    ("ZWL005892", ZWL005892, "Jammu Barnai, Jammu", (32.755253, 74.825204));
    ("ZWL008753", ZWL008753, "Jammu Gandhi Nagar, Jammu", (32.704392, 74.864830));
    ("ZWL008047", ZWL008047, "Jammu OLD JAMMU, Jammu", (32.727614, 74.856395));
    ("ZWL002687", ZWL002687, "Jammu Channi Himmat, Jammu", (32.690740, 74.886902));
    ("ZWL003195", ZWL003195, "Raipur Shankar Nagar, Raipur", (21.251392, 81.663850));
    ("ZWL009896", ZWL009896, "Raipur Purena, Raipur", (21.235636, 81.692460));
    ("ZWL001038", ZWL001038, "Raipur Mowa, Raipur", (21.272467, 81.671100));
    ("ZWL008872", ZWL008872, "Raipur Mahaveer Nagar", (21.210730, 81.640523));
    ("ZWL004310", ZWL004310, "Raipur Samta Colony, Raipur", (21.243164, 81.621252));
    ("ZWL006651", ZWL006651, "Raipur Civil Lines, Raipur", (21.243402, 81.650848));
    ("ZWL008695", ZWL008695, "Raipur Devendra Nagar", (21.252033, 81.650070));
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


    #[test]
    fn test_locality() {
        let api_key = fs::read_to_string("target/api_key")
            .expect("Should have been able to read the file");
        let variable = WeatherUnion::from_key(api_key);
        let out = aw!(variable.locality(LocalityId::ZWL003467)); // Banashankari, BLR
        drop(variable);
        println!("locality {:?}", out);
        assert!(out.is_ok());

    }
}
