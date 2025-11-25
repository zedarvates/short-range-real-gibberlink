"""
RealGibber - Secure Directional Communication Protocol Suite

A comprehensive Python library for secure directional communication protocols
designed for autonomous systems, including drone operations and industrial automation.

This package provides Python bindings to the Rust core library, enabling:
- Secure directional communication using ultrasonic, visual, and laser channels
- Real-time range detection and environmental adaptation
- Cryptographic security with post-quantum resistance
- Mission planning and execution with weather intelligence
- Comprehensive audit trails and compliance reporting

Example usage:
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
    weather.update_weather(WeatherData(...))
"""

from ._core import (
    # Core cryptographic components
    PyCryptoEngine as CryptoEngine,
    PyVisualEngine as VisualEngine,
    PyVisualPayload as VisualPayload,
    PyAudioEngine as AudioEngine,
    PyProtocolEngine as ProtocolEngine,
    PyRgibberLink as RgibberLink,

    # Range detection and laser communication
    PyRangeDetector as RangeDetector,
    PyRangeMeasurement as RangeMeasurement,
    PyRangeEnvironmentalConditions as RangeEnvironmentalConditions,
    PyLaserEngine as LaserEngine,
    PyAlignmentStatus as AlignmentStatus,
    PyUltrasonicBeamEngine as UltrasonicBeamEngine,
    PyOpticalECC as OpticalECC,

    # Channel validation and security
    PyChannelValidator as ChannelValidator,
    PyChannelData as ChannelData,
    PySecurityManager as SecurityManager,

    # Performance monitoring
    PyPerformanceMonitor as PerformanceMonitor,
    PyBenchmarkResult as BenchmarkResult,
    PyPerformanceMetrics as PerformanceMetrics,

    # Post-quantum cryptography
    PyPostQuantumEngine as PostQuantumEngine,
    PyKyberCiphertextData as KyberCiphertextData,

    # Weather and mission management
    PyWeatherManager as WeatherManager,
    PyWeatherData as WeatherData,
    PyGeoCoordinate as GeoCoordinate,
    PyWeatherImpact as WeatherImpact,
    PyWindImpact as WindImpact,
    PyValidationResult as ValidationResult,
    PyConstraintViolation as ConstraintViolation,
    PyWeatherAdaptation as WeatherAdaptation,
    PyRiskAssessment as RiskAssessment,
    PyMissionPayload as MissionPayload,
    PyMissionHeader as MissionHeader,
    PyMissionTask as MissionTask,
    PyDroneSpecifications as DroneSpecifications,

    # Audit and compliance
    PyAuditSystem as AuditSystem,
    PyAuditEntry as AuditEntry,
    PySecurityAlert as SecurityAlert,
)

__version__ = "0.3.0"
__author__ = "RealGibber Team"
__email__ = "contact@realgibber.com"
__license__ = "GPL-3.0"

__all__ = [
    # Core components
    "CryptoEngine", "VisualEngine", "VisualPayload", "AudioEngine",
    "ProtocolEngine", "RgibberLink",

    # Range detection and communication
    "RangeDetector", "RangeMeasurement", "RangeEnvironmentalConditions",
    "LaserEngine", "AlignmentStatus", "UltrasonicBeamEngine", "OpticalECC",

    # Security and validation
    "ChannelValidator", "ChannelData", "SecurityManager",

    # Performance monitoring
    "PerformanceMonitor", "BenchmarkResult", "PerformanceMetrics",

    # Post-quantum cryptography
    "PostQuantumEngine", "KyberCiphertextData",

    # Weather and mission management
    "WeatherManager", "WeatherData", "GeoCoordinate", "WeatherImpact",
    "WindImpact", "ValidationResult", "ConstraintViolation",
    "WeatherAdaptation", "RiskAssessment", "MissionPayload",
    "MissionHeader", "MissionTask", "DroneSpecifications",

    # Audit and compliance
    "AuditSystem", "AuditEntry", "SecurityAlert",
]