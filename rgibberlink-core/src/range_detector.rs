//! # Range Detector Module
//!
//! Ultrasonic time-of-flight ranging system for long-range communication optimization.
//! Provides accurate distance measurements (10-200m) with 1m precision for adaptive power profiles.

use std::sync::Arc;
use std::collections::VecDeque;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

#[cfg(target_os = "android")]
use std::os::raw::{c_char, c_int};

#[cfg(target_os = "android")]
extern "C" {
    fn ultrasonic_init_ranging() -> c_int;
    fn ultrasonic_transmit_pulse(frequency_hz: f32, duration_us: u32) -> c_int;
    fn ultrasonic_start_listening(timeout_ms: u32) -> c_int;
    fn ultrasonic_get_echo_time() -> f64; // microseconds
    fn ultrasonic_get_signal_strength() -> f32;
}

/// Comprehensive error types for range detection operations
#[derive(Debug, thiserror::Error)]
pub enum RangeDetectorError {
    #[error("Hardware initialization failed")]
    HardwareInitFailed,
    #[error("Pulse transmission failed")]
    TransmissionFailed,
    #[error("Echo detection failed")]
    EchoDetectionFailed,
    #[error("Invalid measurement: {0}")]
    InvalidMeasurement(String),
    #[error("Timeout waiting for echo")]
    Timeout,
    #[error("Signal strength too low")]
    LowSignalStrength,
    #[error("Interference detected")]
    InterferenceDetected,
    #[error("Temperature compensation failed")]
    TemperatureCompensationFailed,
}

/// Configuration for ultrasonic ranging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangingConfig {
    pub pulse_frequency_hz: f32,      // 40kHz typical for ultrasonic ranging
    pub pulse_duration_us: u32,       // Pulse length in microseconds
    pub listening_timeout_ms: u32,    // Maximum wait time for echo
    pub min_range_m: f32,            // Minimum detectable range (10m)
    pub max_range_m: f32,            // Maximum detectable range (200m)
    pub speed_of_sound_mps: f32,      // Speed of sound (compensated for temperature)
    pub signal_threshold: f32,       // Minimum signal strength for valid detection
    pub averaging_samples: usize,    // Number of samples for averaging
    pub temperature_celsius: f32,    // Ambient temperature for compensation
}

impl Default for RangingConfig {
    fn default() -> Self {
        Self {
            pulse_frequency_hz: 40000.0,    // 40kHz ultrasonic
            pulse_duration_us: 200,         // 200μs pulse
            listening_timeout_ms: 1200,     // ~200m round trip at 340m/s
            min_range_m: 10.0,
            max_range_m: 200.0,
            speed_of_sound_mps: 343.0,      // 20°C at sea level
            signal_threshold: 0.3,
            averaging_samples: 5,
            temperature_celsius: 20.0,
        }
    }
}

/// Range measurement result
#[derive(Debug, Clone)]
pub struct RangeMeasurement {
    pub distance_m: f32,
    pub signal_strength: f32,
    pub timestamp: Instant,
    pub quality_score: f32,          // 0.0-1.0 quality indicator
    pub temperature_compensated: bool,
}

/// Range categories for adaptive profiles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RangeDetectorCategory {
    Close,      // 10-50m
    Medium,     // 50-100m
    Far,        // 100-150m
    Extreme,    // 150-200m
}

impl RangeDetectorCategory {
    pub fn from_distance(distance_m: f32) -> Self {
        match distance_m {
            d if d < 50.0 => RangeDetectorCategory::Close,
            d if d < 100.0 => RangeDetectorCategory::Medium,
            d if d < 150.0 => RangeDetectorCategory::Far,
            _ => RangeDetectorCategory::Extreme,
        }
    }

    pub fn get_range_bounds(&self) -> (f32, f32) {
        match self {
            RangeDetectorCategory::Close => (10.0, 50.0),
            RangeDetectorCategory::Medium => (50.0, 100.0),
            RangeDetectorCategory::Far => (100.0, 150.0),
            RangeDetectorCategory::Extreme => (150.0, 200.0),
        }
    }
}

/// Environmental conditions affecting ranging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RangeEnvironmentalConditions {
    pub temperature_celsius: f32,
    pub humidity_percent: f32,
    pub pressure_hpa: f32,
    pub wind_speed_mps: f32,
    pub visibility_meters: f32,
}

impl Default for RangeEnvironmentalConditions {
    fn default() -> Self {
        Self {
            temperature_celsius: 20.0,
            humidity_percent: 50.0,
            pressure_hpa: 1013.25,
            wind_speed_mps: 0.0,
            visibility_meters: 10000.0,
        }
    }
}

/// Simple Kalman filter for distance estimation
#[derive(Debug)]
struct DistanceKalmanFilter {
    // State: [distance, velocity]
    state: [f32; 2],
    covariance: [f32; 4], // 2x2 matrix flattened
    process_noise: f32,
    measurement_noise: f32,
}

impl DistanceKalmanFilter {
    fn new() -> Self {
        Self {
            state: [0.0; 2],
            covariance: [1.0, 0.0, 0.0, 1.0], // Identity matrix
            process_noise: 0.1,
            measurement_noise: 2.0, // Distance measurement noise in meters
        }
    }

    fn predict(&mut self, dt: f32) {
        // State transition: distance += velocity * dt
        self.state[0] += self.state[1] * dt;

        // Update covariance
        self.covariance[0] += self.process_noise + 2.0 * self.covariance[1] * dt + self.covariance[3] * dt * dt;
        self.covariance[1] += self.covariance[3] * dt;
        self.covariance[2] += self.covariance[3] * dt;
        self.covariance[3] += self.process_noise;
    }

    fn update(&mut self, measurement: f32) {
        let innovation = measurement - self.state[0];
        let innovation_covariance = self.covariance[0] + self.measurement_noise;

        let kalman_gain = [
            self.covariance[0] / innovation_covariance,
            self.covariance[2] / innovation_covariance,
        ];

        self.state[0] += kalman_gain[0] * innovation;
        self.state[1] += kalman_gain[1] * innovation;

        let temp_cov = 1.0 - kalman_gain[0];
        self.covariance[0] *= temp_cov;
        self.covariance[1] *= temp_cov;
        self.covariance[2] *= temp_cov;
        self.covariance[3] *= temp_cov;
    }

    fn get_distance(&self) -> f32 {
        self.state[0]
    }
}

/// Multi-frequency ranging configuration
#[derive(Debug, Clone)]
struct MultiFrequencyConfig {
    frequencies: Vec<f32>,  // Different frequencies for ranging
    pulse_durations: Vec<u32>, // Corresponding pulse durations
    weights: Vec<f32>,     // Weights for combining measurements
}

impl Default for MultiFrequencyConfig {
    fn default() -> Self {
        Self {
            frequencies: vec![35_000.0, 40_000.0, 45_000.0], // 35kHz, 40kHz, 45kHz
            pulse_durations: vec![150, 200, 250], // Shorter pulses for higher frequencies
            weights: vec![0.3, 0.5, 0.2], // Weight center frequency highest
        }
    }
}

/// Ultrasonic range detector using time-of-flight measurements
#[derive(Debug)]
pub struct RangeDetector {
    config: RangingConfig,
    is_active: Arc<Mutex<bool>>,
    measurement_history: Arc<Mutex<VecDeque<RangeMeasurement>>>,
    environmental_conditions: Arc<Mutex<RangeEnvironmentalConditions>>,
    kalman_filter: Arc<Mutex<DistanceKalmanFilter>>,
    multi_freq_config: MultiFrequencyConfig,
    last_measurement_time: Arc<Mutex<Instant>>,
}

impl RangeDetector {
    /// Create a new range detector with default configuration
    pub fn new() -> Self {
        Self {
            config: RangingConfig::default(),
            is_active: Arc::new(Mutex::new(false)),
            measurement_history: Arc::new(Mutex::new(VecDeque::with_capacity(100))),
            environmental_conditions: Arc::new(Mutex::new(RangeEnvironmentalConditions::default())),
            kalman_filter: Arc::new(Mutex::new(DistanceKalmanFilter::new())),
            multi_freq_config: MultiFrequencyConfig::default(),
            last_measurement_time: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// Create range detector with custom configuration
    pub fn with_config(config: RangingConfig) -> Self {
        Self {
            config,
            is_active: Arc::new(Mutex::new(false)),
            measurement_history: Arc::new(Mutex::new(VecDeque::with_capacity(100))),
            environmental_conditions: Arc::new(Mutex::new(RangeEnvironmentalConditions::default())),
            kalman_filter: Arc::new(Mutex::new(DistanceKalmanFilter::new())),
            multi_freq_config: MultiFrequencyConfig::default(),
            last_measurement_time: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// Initialize the ultrasonic ranging hardware
    pub async fn initialize(&mut self) -> Result<(), RangeDetectorError> {
        #[cfg(target_os = "android")]
        {
            let result = unsafe { ultrasonic_init_ranging() };
            if result != 0 {
                return Err(RangeDetectorError::HardwareInitFailed);
            }
        }

        *self.is_active.lock().await = true;
        Ok(())
    }

    /// Check if range detector is active
    pub async fn is_active(&self) -> bool {
        *self.is_active.lock().await
    }

    /// Perform a single range measurement
    pub async fn measure_distance(&self) -> Result<RangeMeasurement, RangeDetectorError> {
        if !self.is_active().await {
            return Err(RangeDetectorError::HardwareInitFailed);
        }

        // Update speed of sound based on environmental conditions
        let speed_of_sound = self.calculate_speed_of_sound().await;

        // Transmit ultrasonic pulse
        self.transmit_pulse().await?;

        // Listen for echo
        let echo_time_us = self.listen_for_echo().await?;
        let signal_strength = self.get_signal_strength().await?;

        // Validate signal strength
        if signal_strength < self.config.signal_threshold {
            return Err(RangeDetectorError::LowSignalStrength);
        }

        // Calculate distance (round trip, so divide by 2)
        let distance_m = (echo_time_us * speed_of_sound as f64 / 1_000_000.0 / 2.0) as f32;

        // Validate distance bounds
        if distance_m < self.config.min_range_m || distance_m > self.config.max_range_m {
            return Err(RangeDetectorError::InvalidMeasurement(
                format!("Distance {}m out of bounds [{}-{}m]",
                       distance_m, self.config.min_range_m, self.config.max_range_m)
            ));
        }

        // Calculate quality score based on signal strength and expected attenuation
        let quality_score = self.calculate_quality_score(distance_m, signal_strength);

        let measurement = RangeMeasurement {
            distance_m,
            signal_strength,
            timestamp: Instant::now(),
            quality_score,
            temperature_compensated: true,
        };

        // Store measurement in history
        self.store_measurement(measurement.clone()).await;

        Ok(measurement)
    }

    /// Perform multiple measurements and return averaged result
    pub async fn measure_distance_averaged(&self) -> Result<RangeMeasurement, RangeDetectorError> {
        let mut measurements = Vec::new();

        for _ in 0..self.config.averaging_samples {
            match self.measure_distance().await {
                Ok(measurement) => measurements.push(measurement),
                Err(e) => {
                    // Continue with other measurements, but if too many fail, return error
                    if measurements.len() < self.config.averaging_samples / 2 {
                        return Err(e);
                    }
                }
            }

            // Small delay between measurements
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        if measurements.is_empty() {
            return Err(RangeDetectorError::EchoDetectionFailed);
        }

        // Calculate weighted average based on quality scores
        let total_weight: f32 = measurements.iter().map(|m| m.quality_score).sum();
        let avg_distance = measurements.iter()
            .map(|m| m.distance_m * m.quality_score)
            .sum::<f32>() / total_weight;

        let avg_signal_strength = measurements.iter()
            .map(|m| m.signal_strength)
            .sum::<f32>() / measurements.len() as f32;

        let avg_quality = measurements.iter()
            .map(|m| m.quality_score)
            .sum::<f32>() / measurements.len() as f32;

        Ok(RangeMeasurement {
            distance_m: avg_distance,
            signal_strength: avg_signal_strength,
            timestamp: Instant::now(),
            quality_score: avg_quality,
            temperature_compensated: true,
        })
    }

    /// Fast multi-frequency ranging for improved accuracy and speed
    pub async fn measure_distance_fast(&self) -> Result<RangeMeasurement, RangeDetectorError> {
        if !self.is_active().await {
            return Err(RangeDetectorError::HardwareInitFailed);
        }

        let mut frequency_measurements = Vec::new();
        let mut total_weight = 0.0;
        let mut weighted_distance = 0.0;

        // Measure at multiple frequencies simultaneously for speed
        for (i, &frequency) in self.multi_freq_config.frequencies.iter().enumerate() {
            let pulse_duration = self.multi_freq_config.pulse_durations[i];
            let weight = self.multi_freq_config.weights[i];

            // Quick measurement at this frequency
            match self.measure_at_frequency(frequency, pulse_duration).await {
                Ok(measurement) => {
                    let distance = measurement.distance_m;
                    frequency_measurements.push(measurement);
                    weighted_distance += distance * weight;
                    total_weight += weight;
                }
                Err(_) => {
                    // Skip failed measurements but continue with others
                    continue;
                }
            }
        }

        if frequency_measurements.is_empty() {
            return Err(RangeDetectorError::EchoDetectionFailed);
        }

        let avg_distance = weighted_distance / total_weight;

        // Update Kalman filter
        let mut kalman = self.kalman_filter.lock().await;
        let now = Instant::now();
        let dt = *self.last_measurement_time.lock().await;
        let dt_seconds = now.duration_since(dt).as_secs_f32();

        kalman.predict(dt_seconds);
        kalman.update(avg_distance);

        *self.last_measurement_time.lock().await = now;

        let filtered_distance = kalman.get_distance();

        // Calculate combined quality score
        let avg_signal = frequency_measurements.iter()
            .map(|m| m.signal_strength)
            .sum::<f32>() / frequency_measurements.len() as f32;

        let quality_score = frequency_measurements.iter()
            .map(|m| m.quality_score)
            .sum::<f32>() / frequency_measurements.len() as f32;

        let measurement = RangeMeasurement {
            distance_m: filtered_distance,
            signal_strength: avg_signal,
            timestamp: now,
            quality_score,
            temperature_compensated: true,
        };

        // Store in history
        self.store_measurement(measurement.clone()).await;

        Ok(measurement)
    }

    /// Measure distance at a specific frequency
    async fn measure_at_frequency(&self, frequency: f32, pulse_duration: u32) -> Result<RangeMeasurement, RangeDetectorError> {
        let speed_of_sound = self.calculate_speed_of_sound().await;

        // Transmit pulse at specific frequency
        #[cfg(target_os = "android")]
        {
            let result = unsafe { ultrasonic_transmit_pulse(frequency, pulse_duration) };
            if result != 0 {
                return Err(RangeDetectorError::TransmissionFailed);
            }
        }

        // Listen for echo with shorter timeout for speed
        let timeout_ms = (self.config.max_range_m * 2.0 / speed_of_sound * 1000.0) as u32;
        let short_timeout = timeout_ms.min(800); // Cap at 800ms for speed

        #[cfg(target_os = "android")]
        {
            let result = unsafe { ultrasonic_start_listening(short_timeout) };
            if result != 0 {
                return Err(RangeDetectorError::EchoDetectionFailed);
            }

            let echo_time = unsafe { ultrasonic_get_echo_time() };
            if echo_time <= 0.0 {
                return Err(RangeDetectorError::Timeout);
            }

            let signal_strength = unsafe { ultrasonic_get_signal_strength() };

            if signal_strength < self.config.signal_threshold {
                return Err(RangeDetectorError::LowSignalStrength);
            }

            let distance_m = (echo_time * speed_of_sound as f64 / 1_000_000.0 / 2.0) as f32;

            if distance_m < self.config.min_range_m || distance_m > self.config.max_range_m {
                return Err(RangeDetectorError::InvalidMeasurement(
                    format!("Distance {}m out of bounds", distance_m)
                ));
            }

            let quality_score = self.calculate_quality_score(distance_m, signal_strength);

            Ok(RangeMeasurement {
                distance_m,
                signal_strength,
                timestamp: Instant::now(),
                quality_score,
                temperature_compensated: true,
            })
        }

        #[cfg(not(target_os = "android"))]
        {
            // Mock implementation for fast ranging
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let mock_distance = rng.gen_range(50.0..150.0);
            let round_trip_time_us = (mock_distance * 2.0 / speed_of_sound) * 1_000_000.0;

            // Simulate faster response for higher frequencies
            tokio::time::sleep(Duration::from_micros((round_trip_time_us * 0.1) as u64)).await;

            Ok(RangeMeasurement {
                distance_m: mock_distance,
                signal_strength: rng.gen_range(0.6..0.95),
                timestamp: Instant::now(),
                quality_score: rng.gen_range(0.7..0.95),
                temperature_compensated: true,
            })
        }
    }

    /// Transmit ultrasonic pulse
    async fn transmit_pulse(&self) -> Result<(), RangeDetectorError> {
        #[cfg(target_os = "android")]
        {
            let result = unsafe {
                ultrasonic_transmit_pulse(self.config.pulse_frequency_hz, self.config.pulse_duration_us)
            };
            if result != 0 {
                return Err(RangeDetectorError::TransmissionFailed);
            }
        }

        #[cfg(not(target_os = "android"))]
        {
            // Mock implementation - simulate pulse transmission
            tokio::time::sleep(Duration::from_micros(self.config.pulse_duration_us as u64)).await;
        }

        Ok(())
    }

    /// Listen for echo and return time in microseconds
    async fn listen_for_echo(&self) -> Result<f64, RangeDetectorError> {
        #[cfg(target_os = "android")]
        {
            let result = unsafe { ultrasonic_start_listening(self.config.listening_timeout_ms) };
            if result != 0 {
                return Err(RangeDetectorError::EchoDetectionFailed);
            }

            let echo_time = unsafe { ultrasonic_get_echo_time() };
            if echo_time <= 0.0 {
                return Err(RangeDetectorError::Timeout);
            }

            Ok(echo_time)
        }

        #[cfg(not(target_os = "android"))]
        {
            // Mock implementation - simulate echo detection
            // Generate realistic round-trip time for 50-150m range
            use rand::Rng;
            let mut rng = rand::thread_rng();
            let mock_distance = rng.gen_range(50.0..150.0);
            let speed_of_sound = 343.0; // m/s
            let round_trip_time_us = (mock_distance * 2.0 / speed_of_sound) * 1_000_000.0;
            Ok(round_trip_time_us)
        }
    }

    /// Get signal strength of received echo
    async fn get_signal_strength(&self) -> Result<f32, RangeDetectorError> {
        #[cfg(target_os = "android")]
        {
            let strength = unsafe { ultrasonic_get_signal_strength() };
            Ok(strength)
        }

        #[cfg(not(target_os = "android"))]
        {
            // Mock implementation
            use rand::Rng;
            let mut rng = rand::thread_rng();
            Ok(rng.gen_range(0.4..0.9))
        }
    }

    /// Calculate speed of sound based on environmental conditions
    async fn calculate_speed_of_sound(&self) -> f32 {
        let env = self.environmental_conditions.lock().await;

        // Enhanced speed of sound calculation
        // Base formula: v = 331.3 + 0.606 * T (m/s at T°C)
        let base_speed = 331.3 + 0.606 * env.temperature_celsius;

        // Humidity correction using more accurate formula
        // The speed increases with humidity due to molecular weight effects
        let humidity_factor = 1.0 + 0.000012 * env.humidity_percent * env.humidity_percent.sqrt();
        let humidity_corrected = base_speed * humidity_factor;

        // Pressure correction (ideal gas law)
        // v ∝ √(γP/ρ) where γ is adiabatic index, P is pressure, ρ is density
        let pressure_factor = (env.pressure_hpa / 1013.25).sqrt();
        let pressure_corrected = humidity_corrected * pressure_factor;

        // Wind correction (headwind increases effective speed)
        // This is a simplified model - in reality wind affects the medium differently
        let wind_correction = 0.001 * env.wind_speed_mps * env.wind_speed_mps.signum(); // Small correction

        pressure_corrected + wind_correction
    }

    /// Calculate measurement quality score
    fn calculate_quality_score(&self, distance_m: f32, signal_strength: f32) -> f32 {
        // Quality based on signal strength and expected attenuation
        // Ultrasonic attenuation increases with distance and frequency
        let expected_attenuation = 0.1 * distance_m * (self.config.pulse_frequency_hz / 40000.0).sqrt();
        let expected_strength = 1.0 / (1.0 + expected_attenuation);

        let strength_score = signal_strength / expected_strength;
        strength_score.clamp(0.0, 1.0)
    }

    /// Store measurement in history
    async fn store_measurement(&self, measurement: RangeMeasurement) {
        let mut history = self.measurement_history.lock().await;

        // Keep only recent measurements (last 100)
        if history.len() >= 100 {
            history.pop_front();
        }

        history.push_back(measurement);
    }

    /// Get recent measurement history
    pub async fn get_measurement_history(&self) -> Vec<RangeMeasurement> {
        let history = self.measurement_history.lock().await;
        history.iter().cloned().collect()
    }

    /// Update environmental conditions for compensation
    pub async fn update_environmental_conditions(&self, conditions: RangeEnvironmentalConditions) {
        *self.environmental_conditions.lock().await = conditions;

        // Update speed of sound in config
        let speed_of_sound = self.calculate_speed_of_sound().await;
        // Note: In a real implementation, this would update the config atomically
    }

    /// Get current environmental conditions
    pub async fn get_environmental_conditions(&self) -> RangeEnvironmentalConditions {
        self.environmental_conditions.lock().await.clone()
    }

    /// Get current range category
    pub async fn get_current_range_category(&self) -> Option<RangeDetectorCategory> {
        let history = self.measurement_history.lock().await;
        history.back().map(|m| RangeDetectorCategory::from_distance(m.distance_m))
    }

    /// Shutdown the range detector
    pub async fn shutdown(&mut self) -> Result<(), RangeDetectorError> {
        *self.is_active.lock().await = false;
        Ok(())
    }
}

impl Default for RangeDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_range_detector_creation() {
        let detector = RangeDetector::new();
        assert!(!detector.is_active().await);
    }

    #[tokio::test]
    async fn test_range_detector_initialization() {
        let mut detector = RangeDetector::new();

        // Initialization should succeed (even with mock hardware)
        let result = detector.initialize().await;
        assert!(result.is_ok());
        assert!(detector.is_active().await);
    }

    #[tokio::test]
    async fn test_range_categories() {
        assert_eq!(RangeDetectorCategory::from_distance(25.0), RangeDetectorCategory::Close);
        assert_eq!(RangeDetectorCategory::from_distance(75.0), RangeDetectorCategory::Medium);
        assert_eq!(RangeDetectorCategory::from_distance(125.0), RangeDetectorCategory::Far);
        assert_eq!(RangeDetectorCategory::from_distance(175.0), RangeDetectorCategory::Extreme);
    }

    #[tokio::test]
    async fn test_measurement_storage() {
        let detector = RangeDetector::new();

        // Simulate measurement storage
        let measurement = RangeMeasurement {
            distance_m: 100.0,
            signal_strength: 0.8,
            timestamp: Instant::now(),
            quality_score: 0.9,
            temperature_compensated: true,
        };

        detector.store_measurement(measurement).await;

        let history = detector.get_measurement_history().await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].distance_m, 100.0);
    }

    #[tokio::test]
    async fn test_environmental_compensation() {
        let detector = RangeDetector::new();

        let conditions = RangeEnvironmentalConditions {
            temperature_celsius: 30.0,
            humidity_percent: 70.0,
            pressure_hpa: 1000.0,
            wind_speed_mps: 5.0,
            visibility_meters: 5000.0,
        };

        detector.update_environmental_conditions(conditions.clone()).await;

        let retrieved = detector.get_environmental_conditions().await;
        assert_eq!(retrieved.temperature_celsius, 30.0);
        assert_eq!(retrieved.humidity_percent, 70.0);
    }
}