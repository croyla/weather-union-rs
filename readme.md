# weather-union rust implementation
A rust implementation lib for weatherunion api.
## Quickstart
```rust
let instance = WeatherUnion::from_key("api_key".to_string());
let weather_info = instance.locality(LocalityId::ZWL#).await.unwrap();
println!("Live temperature for {}", LocalityId::ZWL#.locality_name());
println!("{}", weather_info.temperature);
```
Where "api_key" is your WeatherUnion api key and ZWL# is a locality id from [here](https://github.com/croyla/weather-union-rs/blob/master/localities.txt)