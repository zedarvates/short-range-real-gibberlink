# RealGibber Python Bindings

Python bindings for RealGibber - Secure Directional Communication Protocol Suite for autonomous systems.

## Installation

### From PyPI (Coming Soon)
```bash
pip install realgibber
```

### From Source
```bash
# Clone the repository
git clone https://github.com/your-org/realgibber.git
cd realgibber/rgibberlink-core

# Install build dependencies
pip install maturin

# Build and install
maturin develop
```

### Development Installation
```bash
# For development with live reloading
maturin develop --release
```

## Quick Start

```python
from realgibber import RangeDetector, LaserEngine, WeatherManager

# Initialize range detector for distance measurement
detector = RangeDetector()
detector.initialize()

# Measure distance with environmental compensation
measurement = detector.measure_distance()
print(f"Distance: {measurement.distance_m:.2f}m")

# Set up laser communication
laser = LaserEngine("Visible", "OOK", 50.0, 100.0)
laser.initialize()

# Weather-aware mission planning
weather = WeatherManager(100)
# ... weather integration code
```

## Core Components

### Range Detection
Ultrasonic time-of-flight ranging with environmental compensation:

```python
from realgibber import RangeDetector, RangeEnvironmentalConditions

detector = RangeDetector()
detector.initialize()

# Set environmental conditions for accurate measurements
env_conditions = RangeEnvironmentalConditions(
    temperature_celsius=22.5,
    humidity_percent=65.0,
    pressure_hpa=1013.25,
    wind_speed_mps=2.1,
    visibility_meters=15000.0
)
detector.update_environmental_conditions(env_conditions)

# Measure distance
measurement = detector.measure_distance()
print(f"Distance: {measurement.distance_m:.2f}m")
print(f"Quality: {measurement.quality_score:.3f}")
```

### Laser Communication
High-speed optical data transmission with adaptive power control:

```python
from realgibber import LaserEngine, AlignmentStatus

# Initialize laser engine
laser = LaserEngine(
    laser_type="Visible",
    modulation_scheme="OOK",
    max_power_mw=50.0,
    range_meters=100.0
)
laser.initialize()

# Check alignment status
alignment = laser.get_alignment_status()
print(f"Aligned: {alignment.is_aligned}")

# Enable adaptive mode with range detector
range_detector = RangeDetector()
range_detector.initialize()
laser.enable_adaptive_mode(range_detector)

# Transmit data
laser.transmit_data(b"Hello, World!")
```

### Weather Integration
Real-time weather assessment and mission validation:

```python
from realgibber import WeatherManager, WeatherData, GeoCoordinate

weather_mgr = WeatherManager(max_stations=50)

# Create weather data
location = GeoCoordinate(latitude=45.5231, longitude=-122.6765, altitude_msl=0.0)
weather_data = WeatherData(
    timestamp=time.time(),
    location=location,
    temperature_celsius=18.5,
    humidity_percent=72.0,
    wind_speed_mps=3.2,
    # ... other weather parameters
)

weather_mgr.update_weather(weather_data)

# Assess impact on drone operations
impact = weather_mgr.assess_weather_impact(mission, drone_specs)
print(f"Risk Score: {impact.overall_risk_score:.2f}")
```

### Performance Monitoring
Real-time performance tracking and benchmarking:

```python
from realgibber import PerformanceMonitor

monitor = PerformanceMonitor(max_history=100)

# Run benchmark suite
benchmarks = monitor.run_benchmark_suite(duration_secs=5)
for benchmark in benchmarks:
    print(f"{benchmark.benchmark_type}: {benchmark.throughput_mbps:.2f} Mbps")

# Get current metrics
metrics = monitor.get_current_metrics()
if metrics:
    print(f"CPU Usage: {metrics.cpu_usage_percent:.1f}%")
```

### Security Management
Permission-based access control and authentication:

```python
from realgibber import SecurityManager

security = SecurityManager("High")

# Validate PIN
security.validate_pin("1234")

# Check permissions
security.check_permission("Read", "Local")
security.check_permission("Execute", "Network")
```

## Advanced Usage

### Mission Transfer Protocol
Secure dual-channel mission transfer with human validation:

```python
from realgibber import MissionPayload, MissionTransferProtocol

# Create encrypted mission payload
mission = MissionPayload("Recon Mission", mission_id_bytes)
transfer_protocol = MissionTransferProtocol()

# Transfer via dual channels (ultrasonic + laser)
success = transfer_protocol.transfer_mission(mission, target_device)
```

### Channel Validation
Coupled channel authentication for security:

```python
from realgibber import ChannelValidator, ChannelData

validator = ChannelValidator()

# Validate coupled channels
laser_data = ChannelData("Laser", laser_bytes, timestamp, quality)
ultrasound_data = ChannelData("Ultrasound", audio_bytes, timestamp, quality)

validator.receive_channel_data(laser_data)
validator.receive_channel_data(ultrasound_data)
```

### Audit System
Comprehensive compliance logging:

```python
from realgibber import AuditSystem, AuditEntry

audit = AuditSystem(max_entries=1000)

entry = AuditEntry("MissionTransfer", "High", "Operator", "transfer", True)
audit.record_event(entry)

alerts = audit.get_active_alerts()
```

## Hardware Requirements

### Range Detection
- Ultrasonic transducer (40kHz carrier frequency)
- Audio interface with low-latency support
- Environmental sensors (temperature, humidity, pressure)

### Laser Communication
- Laser diode module (visible/IR, 10-100mW)
- Photodiode or camera for reception
- Beam steering system (optional)
- Power management circuitry

### General Requirements
- Python 3.8+
- Audio hardware for ultrasonic communication
- Camera for QR code scanning and laser alignment
- GPS for location-based features

## Performance Characteristics

### Range Detection
- **Accuracy**: ±5cm in optimal conditions
- **Range**: 0.1m - 50m (environmental dependent)
- **Update Rate**: 10Hz continuous measurement
- **Environmental Compensation**: Temperature, humidity, pressure, wind

### Laser Communication
- **Data Rate**: 1-10 Mbps (OOK modulation)
- **Range**: 10-200m (line-of-sight)
- **Modulation**: OOK, PWM, QR projection, FSK
- **Adaptive Power**: 1-100mW based on range

### Security
- **Cryptography**: AES-GCM-256, ECDH key exchange
- **Authentication**: Coupled channel validation
- **Audit**: SOC 2, GDPR, HIPAA compliant

## Examples

See the `examples/` directory for comprehensive usage examples:

- `python_range_detection_demo.py` - Complete range detection and laser communication demo
- `mission_transfer_example.py` - Secure mission transfer workflow
- `weather_integration_demo.py` - Weather-aware mission planning

## API Reference

### Core Classes

#### RangeDetector
- `initialize()` - Initialize the range detector
- `measure_distance()` - Single distance measurement
- `measure_distance_averaged(samples)` - Averaged measurement for accuracy
- `update_environmental_conditions(conditions)` - Update environmental parameters
- `get_current_range_category()` - Get current range classification

#### LaserEngine
- `initialize()` - Initialize laser communication
- `transmit_data(data)` - Transmit data via laser
- `get_alignment_status()` - Check beam alignment
- `enable_adaptive_mode(range_detector)` - Enable adaptive power control

#### WeatherManager
- `update_weather(weather_data)` - Update weather conditions
- `assess_weather_impact(mission, drone_specs)` - Assess weather impact
- `validate_mission_constraints(mission, drone_specs)` - Validate mission feasibility

#### PerformanceMonitor
- `run_benchmark_suite(duration)` - Run performance benchmarks
- `get_current_metrics()` - Get current performance metrics

#### SecurityManager
- `validate_pin(pin)` - Validate PIN authentication
- `check_permission(permission, scope)` - Check access permissions

## Troubleshooting

### Common Issues

1. **Range Detection Not Working**
   - Check audio hardware permissions
   - Verify ultrasonic transducer connection
   - Update environmental conditions

2. **Laser Communication Failing**
   - Check camera permissions for alignment
   - Verify laser hardware availability
   - Ensure line-of-sight conditions

3. **Weather Data Not Updating**
   - Check network connectivity
   - Verify API keys for weather services
   - Update location permissions

### Hardware Compatibility

- **Android**: Full hardware acceleration support
- **Linux**: ALSA audio system compatibility
- **macOS**: Core Audio framework support
- **Windows**: WASAPI audio interface support

## Contributing

Contributions welcome! Please see the main project documentation for contribution guidelines.

## License

GPL-3.0 - See LICENSE file for details.

## Support

- **Documentation**: https://github.com/your-org/realgibber/tree/main/Documentations
- **Issues**: https://github.com/your-org/realgibber/issues
- **Discussions**: https://github.com/your-org/realgibber/discussions