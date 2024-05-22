## weather-union rust implementation
```rust
let instance = WeatherUnion::from_key("api_key".to_string());
let weather_info = instance.locality_id("locality_id".to_string()).await.unwrap();
println!("{}", weather_info.temperature);
```
Where "api_key" is your WeatherUnion api key and "locality_id" is a locality id from [here](https://b.zmtcdn.com/data/file_assets/65fa362da3aa560a92f0b8aeec0dfda31713163042.pdf)