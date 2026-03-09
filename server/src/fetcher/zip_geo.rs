use std::collections::HashMap;
use std::path::PathBuf;

const CSV_URL: &str =
    "https://raw.githubusercontent.com/midwire/free_zipcode_data/develop/all_us_zipcodes.csv";
pub const EARTH_RADIUS_KM: f64 = 6371.0;

pub struct ZipGeo {
    coordinates: HashMap<String, (f64, f64)>,
}

impl ZipGeo {
    pub async fn load() -> Self {
        let cache_path = data_cache_path();
        let csv_data = match std::fs::read_to_string(&cache_path) {
            Ok(data) => {
                tracing::info!("Loaded zip centroids from cache: {}", cache_path.display());
                data
            }
            Err(_) => {
                tracing::info!("Downloading zip centroid data from GitHub...");
                match download_csv().await {
                    Ok(data) => {
                        if let Some(parent) = cache_path.parent() {
                            std::fs::create_dir_all(parent).ok();
                        }
                        if let Err(error) = std::fs::write(&cache_path, &data) {
                            tracing::warn!(
                                "Failed to cache zip data to {}: {error}",
                                cache_path.display()
                            );
                        }
                        data
                    }
                    Err(error) => {
                        tracing::error!("Failed to download zip centroid data: {error}");
                        String::new()
                    }
                }
            }
        };

        let coordinates = parse_csv(&csv_data);
        Self { coordinates }
    }

    pub fn lookup(&self, zip: &str) -> Option<(f64, f64)> {
        self.coordinates.get(zip).copied()
    }

    pub fn len(&self) -> usize {
        self.coordinates.len()
    }
}

fn data_cache_path() -> PathBuf {
    PathBuf::from("data/zipcodes.csv")
}

async fn download_csv() -> Result<String, reqwest::Error> {
    let response = reqwest::get(CSV_URL).await?.text().await?;
    Ok(response)
}

fn parse_csv(data: &str) -> HashMap<String, (f64, f64)> {
    let mut map = HashMap::new();

    for line in data.lines().skip(1) {
        if let Some((code, latitude, longitude)) = parse_csv_line(line) {
            map.insert(code, (latitude, longitude));
        }
    }

    map
}

fn parse_csv_line(line: &str) -> Option<(String, f64, f64)> {
    let fields: Vec<&str> = line.split(',').collect();
    if fields.len() < 7 {
        return None;
    }

    let code = fields[0].to_string();
    let latitude = fields[5].parse::<f64>().ok()?;
    let longitude = fields[6].parse::<f64>().ok()?;

    Some((code, latitude, longitude))
}

pub fn haversine_distance_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let delta_lat = (lat2 - lat1).to_radians();
    let delta_lon = (lon2 - lon1).to_radians();

    let lat1_radians = lat1.to_radians();
    let lat2_radians = lat2.to_radians();

    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1_radians.cos() * lat2_radians.cos() * (delta_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS_KM * c
}
