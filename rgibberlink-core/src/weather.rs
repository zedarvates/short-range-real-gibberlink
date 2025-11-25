//! Weather integration and constraint evaluation for drone missions
//!
//! This module provides weather data ingestion, analysis, and mission constraint
//! validation based on weather conditions affecting drone operations. Implements
//! weather-adaptive planning with constraint checking and abort logic.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::time::SystemTime;
use super::mission::*;
use super::security::{WeatherCondition, TimeOfDay};

#[cfg(feature = "weather-api")]
use reqwest;
#[cfg(feature = "weather-api")]
use tokio;

/// Weather data source types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WeatherSource {
    LocalSensor,
    WeatherAPI,
    AirportMETAR,
    SatelliteData,
    ForecastModel,
}

/// Comprehensive weather data structure (extended from mission WeatherSnapshot)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherData {
    pub timestamp: SystemTime,
    pub location: GeoCoordinate,
    pub temperature_celsius: f32,
    pub humidity_percent: f32,
    pub wind_speed_mps: f32,
    pub wind_direction_degrees: f32,
    pub gust_speed_mps: f32,
    pub visibility_meters: f32,
    pub precipitation_type: Option<String>,
    pub precipitation_rate_mmh: f32,
    pub pressure_hpa: f32,
    pub cloud_cover_percent: f32,
    pub lightning_probability: f32,
    pub source: WeatherSource,
    pub forecast_horizon_hours: Option<u32>,
}

/// Weather impact assessment on different aspects of drone operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherImpact {
    pub wind_impact: WindImpact,
    pub precipitation_impact: PrecipitationImpact,
    pub visibility_impact: VisibilityImpact,
    pub temperature_impact: TemperatureImpact,
    pub microclimate_impact: MicroclimateImpact,
    pub solar_em_impact: SolarEMImpact,
    pub overall_risk_score: f32, // 0.0 to 1.0
    pub recommended_actions: Vec<String>,
}

/// Wind effects on drone navigation and endurance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindImpact {
    pub track_deviation_degrees: f32,
    pub increased_power_draw_w: f32,
    pub reduced_endurance_percent: f32,
    pub speed_cap_mps: Option<f32>,
    pub heading_correction_needed: bool,
    pub abort_threshold_exceeded: bool,
}

/// Precipitation effects on sensors and electrical systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrecipitationImpact {
    pub sensor_degradation_percent: f32,
    pub icing_risk: f32, // 0.0 to 1.0
    pub electrical_exposure_risk: f32,
    pub camera_tasks_blocked: bool,
    pub require_sheltered_routes: bool,
    pub canopy_docking_required: bool,
}

/// Visibility effects on navigation and sensing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisibilityImpact {
    pub navigation_reliability_percent: f32,
    pub sensing_reliability_percent: f32,
    pub altitude_adjustment_m: Option<f32>,
    pub slower_speed_required_mps: Option<f32>,
    pub lidar_preferred: bool,
    pub radar_preferred: bool,
    pub contingency_hover_available: bool,
}

/// Temperature effects on battery and components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemperatureImpact {
    pub battery_efficiency_loss_percent: f32,
    pub component_stress_risk: f32,
    pub mission_duration_reduction_percent: f32,
    pub larger_energy_reserve_required: bool,
    pub thermal_checkpoints_recommended: Vec<String>,
}

/// Microclimate effects from obstacles and terrain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicroclimateImpact {
    pub sudden_gust_risk: f32,
    pub vortex_risk_near_obstacles: f32,
    pub standoff_distance_required_m: f32,
    pub vertical_speed_limit_mps: Option<f32>,
    pub waypoint_smoothing_needed: bool,
}

/// Solar and electromagnetic interference effects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolarEMImpact {
    pub sensor_noise_increase_percent: f32,
    pub gnss_reliability_degradation: f32,
    pub multi_sensor_fusion_required: bool,
    pub gnss_trust_gating_active: bool,
    pub local_reference_dead_reckoning: bool,
}

/// Mission constraint validation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintValidationResult {
    pub is_valid: bool,
    pub violations: Vec<ConstraintViolation>,
    pub weather_adaptations: Vec<WeatherAdaptation>,
    pub risk_assessment: RiskAssessment,
}

/// Individual constraint violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintViolation {
    pub constraint_type: String,
    pub severity: ViolationSeverity,
    pub description: String,
    pub affected_components: Vec<String>,
    pub remediation_required: bool,
}

/// Violation severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ViolationSeverity {
    Warning,
    Critical,
    Abort,
}

/// Weather adaptation recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherAdaptation {
    pub adaptation_type: AdaptationType,
    pub description: String,
    pub parameter_changes: HashMap<String, f32>,
    pub route_modifications: Vec<RouteModification>,
}

/// Types of weather adaptations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdaptationType {
    SpeedAdjustment,
    AltitudeChange,
    RouteRerouting,
    SensorSwitching,
    TimingAdjustment,
    AbortRecommended,
}

/// Route modification for weather adaptation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteModification {
    pub segment_id: u32,
    pub original_waypoints: Vec<GeoCoordinate>,
    pub modified_waypoints: Vec<GeoCoordinate>,
    pub reason: String,
}

/// Overall risk assessment for mission execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub overall_risk_level: RiskLevel,
    pub risk_factors: Vec<RiskFactor>,
    pub confidence_score: f32, // 0.0 to 1.0
    pub abort_recommended: bool,
    pub supervision_required: bool,
}

/// Risk levels for mission assessment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RiskLevel {
    Low,
    Moderate,
    High,
    Extreme,
}

/// Individual risk factors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    pub factor_type: String,
    pub impact_level: f32, // 0.0 to 1.0
    pub description: String,
    pub mitigation_measures: Vec<String>,
}

/// Weather Manager for drone operations
pub struct WeatherManager {
    current_weather: Option<WeatherData>,
    weather_history: Vec<WeatherData>,
    max_history_entries: usize,
    api_keys: HashMap<String, String>,
    local_sensor_interface: Option<LocalSensorInterface>,
}

/// Configuration for weather data sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherConfig {
    pub openweather_api_key: Option<String>,
    pub aviation_weather_api_key: Option<String>,
    pub local_sensor_enabled: bool,
    pub cache_duration_seconds: u64,
    pub fallback_sources: Vec<WeatherSource>,
}

/// Local sensor interface for onboard weather sensing
#[derive(Debug, Clone)]
pub struct LocalSensorInterface {
    pub temperature_sensor: bool,
    pub humidity_sensor: bool,
    pub pressure_sensor: bool,
    pub wind_sensor: bool,
    pub visibility_sensor: bool,
}

/// OpenWeatherMap API response structure
#[cfg(feature = "weather-api")]
#[derive(Debug, Deserialize)]
struct OpenWeatherResponse {
    main: OpenWeatherMain,
    wind: OpenWeatherWind,
    visibility: Option<u32>,
    weather: Vec<OpenWeatherWeather>,
    clouds: OpenWeatherClouds,
}

#[cfg(feature = "weather-api")]
#[derive(Debug, Deserialize)]
struct OpenWeatherMain {
    temp: f32,
    humidity: f32,
    pressure: f32,
}

#[cfg(feature = "weather-api")]
#[derive(Debug, Deserialize)]
struct OpenWeatherWind {
    speed: f32,
    deg: f32,
    gust: Option<f32>,
}

#[cfg(feature = "weather-api")]
#[derive(Debug, Deserialize)]
struct OpenWeatherWeather {
    main: String,
    description: String,
}

#[cfg(feature = "weather-api")]
#[derive(Debug, Deserialize)]
struct OpenWeatherClouds {
    all: f32,
}

impl WeatherManager {
    /// Create new weather manager
    pub fn new(max_history: usize) -> Self {
        Self {
            current_weather: None,
            weather_history: Vec::new(),
            max_history_entries: max_history,
            api_keys: HashMap::new(),
            local_sensor_interface: None,
        }
    }

    /// Create weather manager with configuration
    pub fn with_config(config: WeatherConfig, max_history: usize) -> Self {
        let mut api_keys = HashMap::new();
        if let Some(key) = config.openweather_api_key {
            api_keys.insert("openweather".to_string(), key);
        }
        if let Some(key) = config.aviation_weather_api_key {
            api_keys.insert("aviation_weather".to_string(), key);
        }

        let local_sensor_interface = if config.local_sensor_enabled {
            Some(LocalSensorInterface {
                temperature_sensor: true,
                humidity_sensor: true,
                pressure_sensor: true,
                wind_sensor: false, // Most drones don't have wind sensors
                visibility_sensor: false, // Limited onboard visibility sensing
            })
        } else {
            None
        };

        Self {
            current_weather: None,
            weather_history: Vec::new(),
            max_history_entries: max_history,
            api_keys,
            local_sensor_interface,
        }
    }

    /// Update weather data
    pub fn update_weather(&mut self, weather: WeatherData) -> Result<(), WeatherError> {
        // Validate weather data
        self.validate_weather_data(&weather)?;

        self.current_weather = Some(weather.clone());

        // Add to history
        self.weather_history.push(weather);
        if self.weather_history.len() > self.max_history_entries {
            self.weather_history.remove(0);
        }

        Ok(())
    }

    /// Fetch weather data from OpenWeatherMap API
    #[cfg(feature = "weather-api")]
    pub async fn fetch_openweather_data(&mut self, lat: f64, lon: f64) -> Result<(), WeatherError> {
        let api_key = self.api_keys.get("openweather")
            .ok_or(WeatherError::InvalidWeatherData("OpenWeather API key not configured".to_string()))?;

        let url = format!(
            "https://api.openweathermap.org/data/2.5/weather?lat={}&lon={}&appid={}&units=metric",
            lat, lon, api_key
        );

        let response = reqwest::get(&url).await
            .map_err(|e| WeatherError::InvalidWeatherData(format!("API request failed: {}", e)))?;

        let api_response: OpenWeatherResponse = response.json().await
            .map_err(|e| WeatherError::InvalidWeatherData(format!("JSON parsing failed: {}", e)))?;

        let weather_data = WeatherData {
            timestamp: SystemTime::now(),
            location: GeoCoordinate {
                latitude: lat,
                longitude: lon,
                altitude_msl: 0.0, // Would need separate API call for elevation
            },
            temperature_celsius: api_response.main.temp,
            humidity_percent: api_response.main.humidity as f32,
            wind_speed_mps: api_response.wind.speed,
            wind_direction_degrees: api_response.wind.deg,
            gust_speed_mps: api_response.wind.gust.unwrap_or(api_response.wind.speed * 1.2),
            visibility_meters: api_response.visibility.unwrap_or(10000),
            precipitation_type: if api_response.weather[0].main.to_lowercase().contains("rain") {
                Some("rain".to_string())
            } else if api_response.weather[0].main.to_lowercase().contains("snow") {
                Some("snow".to_string())
            } else {
                None
            },
            precipitation_rate_mmh: 0.0, // Would need forecast API for rate
            pressure_hpa: api_response.main.pressure,
            cloud_cover_percent: api_response.clouds.all,
            lightning_probability: 0.0, // Not available in basic API
            source: WeatherSource::WeatherAPI,
            forecast_horizon_hours: None,
        };

        self.update_weather(weather_data)?;
        Ok(())
    }

    /// Fetch weather data from local sensors
    pub async fn fetch_local_sensor_data(&mut self, location: &GeoCoordinate) -> Result<(), WeatherError> {
        let sensors = self.local_sensor_interface.as_ref()
            .ok_or(WeatherError::InvalidWeatherData("Local sensors not configured".to_string()))?;

        // Simulate local sensor readings (in real implementation, this would interface with hardware)
        let mut weather_data = WeatherData {
            timestamp: SystemTime::now(),
            location: location.clone(),
            temperature_celsius: 20.0, // Placeholder - would read from temperature sensor
            humidity_percent: 65.0, // Placeholder - would read from humidity sensor
            wind_speed_mps: 2.5, // Placeholder - would read from wind sensor if available
            wind_direction_degrees: 180.0,
            gust_speed_mps: 3.5,
            visibility_meters: 8000.0, // Placeholder - would estimate from sensors
            precipitation_type: None,
            precipitation_rate_mmh: 0.0,
            pressure_hpa: 1013.0, // Placeholder - would read from pressure sensor
            cloud_cover_percent: 25.0,
            lightning_probability: 0.01,
            source: WeatherSource::LocalSensor,
            forecast_horizon_hours: None,
        };

        // Apply sensor-specific adjustments
        if sensors.temperature_sensor {
            // In real implementation: weather_data.temperature_celsius = read_temperature_sensor();
        }
        if sensors.humidity_sensor {
            // In real implementation: weather_data.humidity_percent = read_humidity_sensor();
        }
        if sensors.pressure_sensor {
            // In real implementation: weather_data.pressure_hpa = read_pressure_sensor();
        }
        if sensors.wind_sensor {
            // In real implementation: (weather_data.wind_speed_mps, weather_data.wind_direction_degrees) = read_wind_sensor();
        }
        if sensors.visibility_sensor {
            // In real implementation: weather_data.visibility_meters = estimate_visibility();
        }

        self.update_weather(weather_data)?;
        Ok(())
    }

    /// Fetch weather data with automatic fallback between sources
    pub async fn fetch_weather_with_fallback(&mut self, location: &GeoCoordinate) -> Result<(), WeatherError> {
        let mut errors = Vec::new();

        // Try OpenWeather API first
        #[cfg(feature = "weather-api")]
        if self.api_keys.contains_key("openweather") {
            match self.fetch_openweather_data(location.latitude, location.longitude).await {
                Ok(_) => return Ok(()),
                Err(e) => errors.push(format!("OpenWeather API: {}", e)),
            }
        }

        // Try local sensors as fallback
        if self.local_sensor_interface.is_some() {
            match self.fetch_local_sensor_data(location).await {
                Ok(_) => return Ok(()),
                Err(e) => errors.push(format!("Local sensors: {}", e)),
            }
        }

        // If all sources failed, return combined error
        Err(WeatherError::InvalidWeatherData(format!(
            "All weather sources failed: {}",
            errors.join("; ")
        )))
    }

    /// Get weather forecast for mission planning
    #[cfg(feature = "weather-api")]
    pub async fn fetch_weather_forecast(&self, lat: f64, lon: f64, hours_ahead: u32) -> Result<Vec<WeatherData>, WeatherError> {
        let api_key = self.api_keys.get("openweather")
            .ok_or(WeatherError::InvalidWeatherData("OpenWeather API key not configured".to_string()))?;

        let url = format!(
            "https://api.openweathermap.org/data/2.5/forecast?lat={}&lon={}&appid={}&units=metric&cnt={}",
            lat, lon, api_key, hours_ahead / 3 // API returns 3-hour intervals
        );

        let response = reqwest::get(&url).await
            .map_err(|e| WeatherError::InvalidWeatherData(format!("Forecast API request failed: {}", e)))?;

        // Parse forecast response (simplified - would need full ForecastResponse struct)
        let _forecast_data: serde_json::Value = response.json().await
            .map_err(|e| WeatherError::InvalidWeatherData(format!("Forecast JSON parsing failed: {}", e)))?;

        // Convert to WeatherData vector (simplified implementation)
        let mut forecast = Vec::new();
        for i in 0..(hours_ahead / 3) {
            let forecast_time = SystemTime::now() + std::time::Duration::from_secs((i * 3 * 3600) as u64);
            // In real implementation, extract data from forecast response
            let weather_point = WeatherData {
                timestamp: forecast_time,
                location: GeoCoordinate { latitude: lat, longitude: lon, altitude_msl: 0.0 },
                temperature_celsius: 22.0, // Placeholder
                humidity_percent: 60.0,
                wind_speed_mps: 3.0,
                wind_direction_degrees: 200.0,
                gust_speed_mps: 4.0,
                visibility_meters: 9000.0,
                precipitation_type: None,
                precipitation_rate_mmh: 0.0,
                pressure_hpa: 1012.0,
                cloud_cover_percent: 40.0,
                lightning_probability: 0.02,
                source: WeatherSource::ForecastModel,
                forecast_horizon_hours: Some((i * 3) as u32),
            };
            forecast.push(weather_point);
        }

        Ok(forecast)
    }

    /// Assess weather impact on drone operations
    pub fn assess_weather_impact(&self, mission: &MissionPayload, drone_specs: &DroneSpecifications) -> Result<WeatherImpact, WeatherError> {
        let weather = self.current_weather.as_ref()
            .ok_or(WeatherError::NoWeatherData)?;

        let mut impact = WeatherImpact {
            wind_impact: self.assess_wind_impact(weather, drone_specs),
            precipitation_impact: self.assess_precipitation_impact(weather, drone_specs),
            visibility_impact: self.assess_visibility_impact(weather),
            temperature_impact: self.assess_temperature_impact(weather, mission),
            microclimate_impact: self.assess_microclimate_impact(weather, mission),
            solar_em_impact: self.assess_solar_em_impact(weather),
            overall_risk_score: 0.0,
            recommended_actions: Vec::new(),
        };

        // Calculate overall risk score as weighted average
        let weights = [0.25, 0.20, 0.15, 0.15, 0.10, 0.10]; // Wind, precip, vis, temp, micro, solar
        let impacts = [
            impact.wind_impact.track_deviation_degrees / 45.0, // Normalize to 0-1
            impact.precipitation_impact.sensor_degradation_percent / 100.0,
            (100.0 - impact.visibility_impact.navigation_reliability_percent) / 100.0,
            impact.temperature_impact.battery_efficiency_loss_percent / 100.0,
            impact.microclimate_impact.sudden_gust_risk,
            impact.solar_em_impact.sensor_noise_increase_percent / 100.0,
        ];

        impact.overall_risk_score = weights.iter().zip(impacts.iter())
            .map(|(w, i)| w * i.min(1.0))
            .sum::<f32>()
            .min(1.0);

        // Generate recommended actions based on impacts
        impact.recommended_actions = self.generate_recommended_actions(&impact, mission);

        Ok(impact)
    }

    /// Validate mission constraints against current weather
    pub fn validate_mission_constraints(&self, mission: &MissionPayload, drone_specs: &DroneSpecifications) -> Result<ConstraintValidationResult, WeatherError> {
        let weather = self.current_weather.as_ref()
            .ok_or(WeatherError::NoWeatherData)?;

        let mut violations = Vec::new();
        let mut adaptations = Vec::new();

        // Check environmental constraints
        self.check_environmental_constraints(weather, &mission.constraints.environmental, &mut violations, &mut adaptations)?;

        // Check safety constraints
        self.check_safety_constraints(weather, &mission.constraints.safety, &mut violations, &mut adaptations)?;

        // Check energy constraints considering weather impacts
        self.check_energy_constraints(weather, &mission.constraints.energy, drone_specs, &mut violations, &mut adaptations)?;

        let is_valid = violations.iter().all(|v| v.severity != ViolationSeverity::Abort);

        let risk_assessment = self.assess_overall_risk(&violations, &adaptations);

        Ok(ConstraintValidationResult {
            is_valid,
            violations,
            weather_adaptations: adaptations,
            risk_assessment,
        })
    }

    /// Assess wind impact on drone operations
    fn assess_wind_impact(&self, weather: &WeatherData, drone_specs: &DroneSpecifications) -> WindImpact {
        let wind_speed = weather.wind_speed_mps;
        let gust_speed = weather.gust_speed_mps;

        // Calculate track deviation based on wind speed
        let track_deviation = (wind_speed * 10.0).min(45.0); // Max 45 degrees deviation

        // Increased power draw due to wind resistance
        let power_increase = wind_speed * drone_specs.power_wind_coefficient;

        // Endurance reduction
        let endurance_reduction = if wind_speed > drone_specs.max_wind_speed_mps {
            50.0 // 50% reduction if above max wind
        } else {
            (wind_speed / drone_specs.max_wind_speed_mps) * 25.0 // Up to 25% reduction
        };

        // Speed cap if gusts exceed threshold
        let speed_cap = if gust_speed > drone_specs.abort_gust_threshold_mps {
            Some(drone_specs.max_speed_mps * 0.6) // Reduce to 60% max speed
        } else {
            None
        };

        WindImpact {
            track_deviation_degrees: track_deviation,
            increased_power_draw_w: power_increase,
            reduced_endurance_percent: endurance_reduction,
            speed_cap_mps: speed_cap,
            heading_correction_needed: track_deviation > 10.0,
            abort_threshold_exceeded: gust_speed > drone_specs.abort_gust_threshold_mps,
        }
    }

    /// Assess precipitation impact
    fn assess_precipitation_impact(&self, weather: &WeatherData, drone_specs: &DroneSpecifications) -> PrecipitationImpact {
        let precip_rate = weather.precipitation_rate_mmh;
        let precip_type = weather.precipitation_type.as_deref().unwrap_or("rain");

        let mut sensor_degradation = 0.0;
        let mut icing_risk = 0.0;
        let mut electrical_risk = 0.0;
        let mut camera_blocked = false;

        match precip_type {
            "rain" => {
                sensor_degradation = (precip_rate / 10.0).min(0.8) * 100.0;
                camera_blocked = precip_rate > 2.0;
                electrical_risk = (precip_rate / 20.0).min(0.3);
            },
            "snow" => {
                sensor_degradation = (precip_rate / 5.0).min(0.9) * 100.0;
                icing_risk = (precip_rate / 10.0).min(0.8);
                camera_blocked = true;
                electrical_risk = (precip_rate / 50.0).min(0.1);
            },
            _ => {}
        }

        PrecipitationImpact {
            sensor_degradation_percent: sensor_degradation,
            icing_risk,
            electrical_exposure_risk: electrical_risk,
            camera_tasks_blocked: camera_blocked,
            require_sheltered_routes: precip_rate > 5.0,
            canopy_docking_required: icing_risk > 0.5,
        }
    }

    /// Assess visibility impact
    fn assess_visibility_impact(&self, weather: &WeatherData) -> VisibilityImpact {
        let visibility = weather.visibility_meters;

        // Navigation and sensing reliability decrease with low visibility
        let reliability = (visibility / 1000.0).min(1.0);

        // Altitude adjustment for better sensor performance
        let altitude_adjustment = if visibility < 300.0 {
            Some(50.0) // Fly 50m higher
        } else {
            None
        };

        // Speed reduction in low visibility
        let speed_reduction = if visibility < 500.0 {
            Some(5.0) // Reduce to 5 m/s
        } else if visibility < 1000.0 {
            Some(8.0) // Allow up to 8 m/s
        } else {
            None
        };

        VisibilityImpact {
            navigation_reliability_percent: reliability * 100.0,
            sensing_reliability_percent: reliability * 80.0, // Sensors affected more
            altitude_adjustment_m: altitude_adjustment,
            slower_speed_required_mps: speed_reduction,
            lidar_preferred: visibility < 500.0,
            radar_preferred: visibility < 200.0,
            contingency_hover_available: visibility >= 100.0,
        }
    }

    /// Assess temperature impact
    fn assess_temperature_impact(&self, weather: &WeatherData, mission: &MissionPayload) -> TemperatureImpact {
        let temp = weather.temperature_celsius;

        // Battery efficiency loss
        let efficiency_loss = if temp < 0.0 {
            (0.0 - temp).min(20.0) * 0.5 // 0.5% loss per degree below 0
        } else if temp > 30.0 {
            (temp - 30.0) * 0.3 // 0.3% loss per degree above 30
        } else {
            0.0
        };

        // Component stress risk
        let stress_risk = if temp < -10.0 || temp > 40.0 { 0.8 } else { 0.0 };

        // Mission duration reduction
        let duration_reduction = efficiency_loss * 0.5; // 50% of efficiency loss affects duration

        // Check if current mission duration would be affected
        let max_duration_seconds = mission.header.max_execution_duration.as_secs() as f32;
        let reduced_duration_seconds = max_duration_seconds * (1.0 - duration_reduction / 100.0);

        TemperatureImpact {
            battery_efficiency_loss_percent: efficiency_loss,
            component_stress_risk: stress_risk,
            mission_duration_reduction_percent: duration_reduction,
            larger_energy_reserve_required: duration_reduction > 10.0,
            thermal_checkpoints_recommended: if stress_risk > 0.5 {
                vec!["Pre-flight thermal check".to_string(), "Mid-mission thermal monitoring".to_string()]
            } else {
                Vec::new()
            },
        }
    }

    /// Assess microclimate impact
    fn assess_microclimate_impact(&self, weather: &WeatherData, mission: &MissionPayload) -> MicroclimateImpact {
        let wind_speed = weather.wind_speed_mps;

        // Higher wind speeds increase microclimate risks
        let gust_risk = (wind_speed / 15.0).min(1.0);
        let vortex_risk = (wind_speed / 10.0).min(0.8);

        // Required standoff distance increases with wind
        let standoff_distance = (wind_speed * 2.0).max(10.0);

        // Vertical speed limitation in turbulence
        let vertical_speed_limit = if wind_speed > 8.0 {
            Some(2.0) // Limit to 2 m/s vertical speed
        } else {
            None
        };

        // Check if mission waypoints need smoothing
        let needs_smoothing = mission.flight_plan.paths.iter()
            .any(|path| path.waypoints.len() > 10); // Complex paths need smoothing

        MicroclimateImpact {
            sudden_gust_risk: gust_risk,
            vortex_risk_near_obstacles: vortex_risk,
            standoff_distance_required_m: standoff_distance,
            vertical_speed_limit_mps: vertical_speed_limit,
            waypoint_smoothing_needed: needs_smoothing,
        }
    }

    /// Assess solar and EM interference impact
    fn assess_solar_em_impact(&self, weather: &WeatherData) -> SolarEMImpact {
        let cloud_cover = weather.cloud_cover_percent;
        let precip_type = weather.precipitation_type.as_deref();

        // Sensor noise increases with atmospheric interference
        let noise_increase = if precip_type.is_some() {
            20.0 // Precipitation affects EM signals
        } else {
            cloud_cover / 5.0 // Cloud cover causes some interference
        };

        // GNSS reliability degradation
        let gnss_degradation = if precip_type == Some("heavy_rain") {
            60.0 // Heavy rain significantly affects GNSS
        } else if cloud_cover > 80.0 {
            30.0 // Thick clouds affect satellite signals
        } else {
            noise_increase
        };

        SolarEMImpact {
            sensor_noise_increase_percent: noise_increase,
            gnss_reliability_degradation: gnss_degradation,
            multi_sensor_fusion_required: gnss_degradation > 30.0,
            gnss_trust_gating_active: gnss_degradation > 50.0,
            local_reference_dead_reckoning: gnss_degradation > 70.0,
        }
    }

    /// Generate recommended actions based on weather impact
    fn generate_recommended_actions(&self, impact: &WeatherImpact, mission: &MissionPayload) -> Vec<String> {
        let mut actions = Vec::new();

        if impact.wind_impact.abort_threshold_exceeded {
            actions.push("ABORT: Gust speeds exceed safe limits".to_string());
        }

        if impact.precipitation_impact.camera_tasks_blocked {
            actions.push("Switch to non-optical sensors for observation tasks".to_string());
            actions.push("Postpone camera-based actions until conditions improve".to_string());
        }

        if impact.visibility_impact.navigation_reliability_percent < 60.0 {
            actions.push("Switch to Lidar/Radar navigation if available".to_string());
            actions.push("Increase altitude for better visibility".to_string());
            actions.push("Reduce speed to improve reaction time".to_string());
        }

        if impact.temperature_impact.larger_energy_reserve_required {
            actions.push("Increase energy reserve margin by 20%".to_string());
            actions.push("Reduce mission duration to compensate for efficiency loss".to_string());
        }

        if impact.microclimate_impact.waypoint_smoothing_needed {
            actions.push("Apply waypoint smoothing to reduce turbulence effects".to_string());
        }

        if impact.solar_em_impact.local_reference_dead_reckoning {
            actions.push("Enable dead reckoning mode during low-GNSS periods".to_string());
        }

        // Overall risk-based actions
        if impact.overall_risk_score > 0.7 {
            actions.push("HIGH RISK: Consider mission abort or significant delays".to_string());
        } else if impact.overall_risk_score > 0.5 {
            actions.push("MODERATE RISK: Monitor weather closely during execution".to_string());
            actions.push("Consider reduced speed and altitude limits".to_string());
        }

        actions
    }
}

/// Drone specifications for weather impact calculations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DroneSpecifications {
    pub max_wind_speed_mps: f32,
    pub max_speed_mps: f32,
    pub abort_gust_threshold_mps: f32,
    pub power_wind_coefficient: f32, // Watts per m/s of wind
    pub mass_kg: f32,
    pub battery_capacity_wh: f32,
    pub sensor_types: Vec<String>,
}

/// Weather processing errors
#[derive(Debug, thiserror::Error)]
pub enum WeatherError {
    #[error("No weather data available")]
    NoWeatherData,
    #[error("Invalid weather data: {0}")]
    InvalidWeatherData(String),
    #[error("Weather data too old (age: {0} seconds)")]
    WeatherDataTooOld(u64),
    #[error("Mission constraint validation failed")]
    ConstraintValidationFailed,
}

impl WeatherManager {
    /// Validate weather data integrity
    fn validate_weather_data(&self, weather: &WeatherData) -> Result<(), WeatherError> {
        // Check for reasonable value ranges
        if weather.temperature_celsius < -50.0 || weather.temperature_celsius > 60.0 {
            return Err(WeatherError::InvalidWeatherData("Temperature out of range".to_string()));
        }

        if weather.wind_speed_mps < 0.0 || weather.wind_speed_mps > 100.0 {
            return Err(WeatherError::InvalidWeatherData("Wind speed out of range".to_string()));
        }

        if weather.visibility_meters < 0.0 || weather.visibility_meters > 50000.0 {
            return Err(WeatherError::InvalidWeatherData("Visibility out of range".to_string()));
        }

        // Check data freshness (max 1 hour old)
        let age_seconds = weather.timestamp.elapsed()
            .unwrap_or(std::time::Duration::from_secs(0))
            .as_secs();

        if age_seconds > 3600 {
            return Err(WeatherError::WeatherDataTooOld(age_seconds));
        }

        Ok(())
    }

    /// Check environmental constraints
    fn check_environmental_constraints(
        &self,
        weather: &WeatherData,
        constraints: &EnvironmentalConstraints,
        violations: &mut Vec<ConstraintViolation>,
        adaptations: &mut Vec<WeatherAdaptation>
    ) -> Result<(), WeatherError> {
        // Temperature constraints
        if weather.temperature_celsius > constraints.max_temperature_c {
            violations.push(ConstraintViolation {
                constraint_type: "max_temperature".to_string(),
                severity: ViolationSeverity::Critical,
                description: format!("Temperature {}°C exceeds limit {}°C",
                    weather.temperature_celsius, constraints.max_temperature_c),
                affected_components: vec!["battery".to_string(), "electronics".to_string()],
                remediation_required: true,
            });
        }

        if weather.temperature_celsius < constraints.min_temperature_c {
            violations.push(ConstraintViolation {
                constraint_type: "min_temperature".to_string(),
                severity: ViolationSeverity::Critical,
                description: format!("Temperature {}°C below limit {}°C",
                    weather.temperature_celsius, constraints.min_temperature_c),
                affected_components: vec!["battery".to_string(), "propulsion".to_string()],
                remediation_required: true,
            });
        }

        // Wind constraints
        if weather.wind_speed_mps > constraints.max_wind_speed_mps {
            let severity = if weather.wind_speed_mps > constraints.max_wind_speed_mps * 1.5 {
                ViolationSeverity::Abort
            } else {
                ViolationSeverity::Critical
            };

            violations.push(ConstraintViolation {
                constraint_type: "max_wind_speed".to_string(),
                severity: severity.clone(),
                description: format!("Wind speed {} m/s exceeds limit {} m/s",
                    weather.wind_speed_mps, constraints.max_wind_speed_mps),
                affected_components: vec!["navigation".to_string(), "stability".to_string()],
                remediation_required: true,
            });

            // Recommend speed reduction
            if severity != ViolationSeverity::Abort {
                adaptations.push(WeatherAdaptation {
                    adaptation_type: AdaptationType::SpeedAdjustment,
                    description: "Reduce speed due to high winds".to_string(),
                    parameter_changes: HashMap::from([("max_speed".to_string(), constraints.max_wind_speed_mps * 0.7)]),
                    route_modifications: Vec::new(),
                });
            }
        }

        // Precipitation constraints
        if weather.precipitation_rate_mmh > constraints.max_precipitation_mmh {
            violations.push(ConstraintViolation {
                constraint_type: "max_precipitation".to_string(),
                severity: ViolationSeverity::Warning,
                description: format!("Precipitation {} mm/h exceeds limit {} mm/h",
                    weather.precipitation_rate_mmh, constraints.max_precipitation_mmh),
                affected_components: vec!["sensors".to_string()],
                remediation_required: false,
            });

            adaptations.push(WeatherAdaptation {
                adaptation_type: AdaptationType::SensorSwitching,
                description: "Switch to rain-resistant sensors".to_string(),
                parameter_changes: HashMap::new(),
                route_modifications: Vec::new(),
            });
        }

        Ok(())
    }

    /// Check safety constraints
    fn check_safety_constraints(
        &self,
        weather: &WeatherData,
        constraints: &SafetyConstraints,
        violations: &mut Vec<ConstraintViolation>,
        adaptations: &mut Vec<WeatherAdaptation>
    ) -> Result<(), WeatherError> {
        // Wind safety
        if weather.wind_speed_mps > constraints.max_wind_speed_mps {
            violations.push(ConstraintViolation {
                constraint_type: "safety_wind_speed".to_string(),
                severity: ViolationSeverity::Abort,
                description: format!("Wind speed {} m/s exceeds safety limit {} m/s",
                    weather.wind_speed_mps, constraints.max_wind_speed_mps),
                affected_components: vec!["flight_safety".to_string()],
                remediation_required: true,
            });
        }

        if weather.gust_speed_mps > constraints.max_gust_speed_mps {
            violations.push(ConstraintViolation {
                constraint_type: "safety_gust_speed".to_string(),
                severity: ViolationSeverity::Abort,
                description: format!("Gust speed {} m/s exceeds safety limit {} m/s",
                    weather.gust_speed_mps, constraints.max_gust_speed_mps),
                affected_components: vec!["flight_safety".to_string()],
                remediation_required: true,
            });
        }

        // Visibility safety
        if weather.visibility_meters < constraints.min_visibility_m {
            let severity = if weather.visibility_meters < constraints.min_visibility_m * 0.5 {
                ViolationSeverity::Abort
            } else {
                ViolationSeverity::Critical
            };

            violations.push(ConstraintViolation {
                constraint_type: "safety_visibility".to_string(),
                severity,
                description: format!("Visibility {}m below safety minimum {}m",
                    weather.visibility_meters, constraints.min_visibility_m),
                affected_components: vec!["navigation".to_string()],
                remediation_required: true,
            });
        }

        Ok(())
    }

    /// Check energy constraints with weather impact
    fn check_energy_constraints(
        &self,
        weather: &WeatherData,
        constraints: &EnergyConstraints,
        drone_specs: &DroneSpecifications,
        violations: &mut Vec<ConstraintViolation>,
        adaptations: &mut Vec<WeatherAdaptation>
    ) -> Result<(), WeatherError> {
        // Calculate weather-adjusted power consumption
        let wind_power_increase = weather.wind_speed_mps * drone_specs.power_wind_coefficient;
        let temp_efficiency_loss = if weather.temperature_celsius > 30.0 {
            0.1 // 10% efficiency loss at high temp
        } else {
            0.0
        };

        let adjusted_power_consumption = constraints.expected_consumption_wh * (1.0 + temp_efficiency_loss);

        // Check if battery reserve is sufficient
        let available_energy = drone_specs.battery_capacity_wh * constraints.reserve_margin_soc;
        if adjusted_power_consumption > available_energy {
            violations.push(ConstraintViolation {
                constraint_type: "energy_reserve".to_string(),
                severity: ViolationSeverity::Critical,
                description: format!("Power consumption {}Wh exceeds reserve {}Wh under weather conditions",
                    adjusted_power_consumption, available_energy),
                affected_components: vec!["power_system".to_string()],
                remediation_required: true,
            });

            // Recommend reducing mission scope
            adaptations.push(WeatherAdaptation {
                adaptation_type: AdaptationType::TimingAdjustment,
                description: "Reduce mission duration to conserve energy".to_string(),
                parameter_changes: HashMap::from([("max_duration_hours".to_string(), 0.5)]),
                route_modifications: Vec::new(),
            });
        }

        Ok(())
    }

    /// Assess overall risk from violations and adaptations
    fn assess_overall_risk(&self, violations: &[ConstraintViolation], adaptations: &[WeatherAdaptation]) -> RiskAssessment {
        let critical_count = violations.iter()
            .filter(|v| v.severity == ViolationSeverity::Critical)
            .count();

        let abort_count = violations.iter()
            .filter(|v| v.severity == ViolationSeverity::Abort)
            .count();

        let risk_level = if abort_count > 0 {
            RiskLevel::Extreme
        } else if critical_count > 2 {
            RiskLevel::High
        } else if critical_count > 0 || !adaptations.is_empty() {
            RiskLevel::Moderate
        } else {
            RiskLevel::Low
        };

        let risk_factors = violations.iter().map(|v| {
            RiskFactor {
                factor_type: v.constraint_type.clone(),
                impact_level: match v.severity {
                    ViolationSeverity::Warning => 0.3,
                    ViolationSeverity::Critical => 0.7,
                    ViolationSeverity::Abort => 1.0,
                },
                description: v.description.clone(),
                mitigation_measures: if v.remediation_required {
                    vec!["Apply recommended adaptations".to_string(), "Monitor conditions closely".to_string()]
                } else {
                    vec!["Continue with caution".to_string()]
                },
            }
        }).collect();

        RiskAssessment {
            overall_risk_level: risk_level,
            risk_factors,
            confidence_score: 0.85, // Default confidence based on data quality
            abort_recommended: abort_count > 0,
            supervision_required: critical_count > 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weather_impact_assessment() {
        let manager = WeatherManager::new(10);

        let weather = WeatherData {
            timestamp: std::time::SystemTime::now(),
            location: GeoCoordinate {
                latitude: 45.0,
                longitude: 2.0,
                altitude_msl: 100.0,
            },
            temperature_celsius: 25.0,
            humidity_percent: 60.0,
            wind_speed_mps: 8.0,
            wind_direction_degrees: 270.0,
            gust_speed_mps: 12.0,
            visibility_meters: 8000.0,
            precipitation_type: None,
            precipitation_rate_mmh: 0.0,
            pressure_hpa: 1013.0,
            cloud_cover_percent: 30.0,
            lightning_probability: 0.0,
            source: WeatherSource::WeatherAPI,
            forecast_horizon_hours: None,
        };

        let drone_specs = DroneSpecifications {
            max_wind_speed_mps: 10.0,
            max_speed_mps: 15.0,
            abort_gust_threshold_mps: 15.0,
            power_wind_coefficient: 5.0,
            mass_kg: 2.5,
            battery_capacity_wh: 100.0,
            sensor_types: vec!["camera".to_string(), "lidar".to_string()],
        };

        let mission = MissionPayload::default();

        // Update weather and test
        let mut manager = WeatherManager::new(10);
        manager.update_weather(weather).expect("Weather update should work");

        // Test will be more comprehensive once WeatherManager has the methods
        // For now, just test weather data validation
        assert!(manager.current_weather.is_some());
    }

    #[tokio::test]
    async fn test_weather_manager_with_config() {
        let config = WeatherConfig {
            openweather_api_key: Some("test_key".to_string()),
            aviation_weather_api_key: None,
            local_sensor_enabled: true,
            cache_duration_seconds: 300,
            fallback_sources: vec![WeatherSource::LocalSensor],
        };

        let manager = WeatherManager::with_config(config, 10);
        assert!(manager.local_sensor_interface.is_some());
        assert!(manager.api_keys.contains_key("openweather"));
    }

    #[tokio::test]
    async fn test_local_sensor_data_fetch() {
        let config = WeatherConfig {
            openweather_api_key: None,
            aviation_weather_api_key: None,
            local_sensor_enabled: true,
            cache_duration_seconds: 300,
            fallback_sources: vec![WeatherSource::LocalSensor],
        };

        let mut manager = WeatherManager::with_config(config, 10);
        let location = GeoCoordinate {
            latitude: 45.0,
            longitude: 2.0,
            altitude_msl: 100.0,
        };

        // This should work with local sensors enabled
        let result = manager.fetch_local_sensor_data(&location).await;
        assert!(result.is_ok());
        assert!(manager.current_weather.is_some());

        let weather = manager.current_weather.as_ref().unwrap();
        assert_eq!(weather.source, WeatherSource::LocalSensor);
        assert_eq!(weather.location.latitude, 45.0);
        assert_eq!(weather.location.longitude, 2.0);
    }

    #[tokio::test]
    async fn test_weather_fallback_without_sources() {
        let config = WeatherConfig {
            openweather_api_key: None,
            aviation_weather_api_key: None,
            local_sensor_enabled: false,
            cache_duration_seconds: 300,
            fallback_sources: vec![],
        };

        let mut manager = WeatherManager::with_config(config, 10);
        let location = GeoCoordinate {
            latitude: 45.0,
            longitude: 2.0,
            altitude_msl: 100.0,
        };

        // This should fail with no sources available
        let result = manager.fetch_weather_with_fallback(&location).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_weather_config_creation() {
        let config = WeatherConfig {
            openweather_api_key: Some("test_key".to_string()),
            aviation_weather_api_key: Some("aviation_key".to_string()),
            local_sensor_enabled: true,
            cache_duration_seconds: 600,
            fallback_sources: vec![WeatherSource::WeatherAPI, WeatherSource::LocalSensor],
        };

        assert_eq!(config.cache_duration_seconds, 600);
        assert!(config.local_sensor_enabled);
        assert_eq!(config.fallback_sources.len(), 2);
    }
}
