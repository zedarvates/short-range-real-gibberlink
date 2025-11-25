#!/usr/bin/env python3
"""
RealGibber Python Range Detection Demo

This script demonstrates the complete range detection and laser communication
capabilities of RealGibber using the Python bindings.

Features demonstrated:
- Ultrasonic time-of-flight ranging
- Environmental compensation
- Laser engine setup and alignment
- Performance monitoring
- Weather-aware operations
"""

import time
import asyncio
from realgibber import (
    RangeDetector, RangeEnvironmentalConditions,
    LaserEngine, AlignmentStatus,
    WeatherManager, WeatherData, GeoCoordinate,
    PerformanceMonitor, BenchmarkResult,
    SecurityManager
)


def demo_range_detection():
    """Demonstrate ultrasonic range detection capabilities."""
    print("🔍 RealGibber Range Detection Demo")
    print("=" * 50)

    # Initialize range detector
    detector = RangeDetector()
    print("📡 Initializing range detector...")

    try:
        detector.initialize()
        print("✅ Range detector initialized successfully")

        # Set up environmental conditions for accurate measurements
        env_conditions = RangeEnvironmentalConditions(
            temperature_celsius=22.5,
            humidity_percent=65.0,
            pressure_hpa=1013.25,
            wind_speed_mps=2.1,
            visibility_meters=15000.0
        )
        detector.update_environmental_conditions(env_conditions)
        print("🌤️ Environmental conditions updated")

        # Perform single measurement
        print("\n📏 Performing single distance measurement...")
        measurement = detector.measure_distance()

        print("📊 Measurement Results:")
        print(".2f")
        print(".3f")
        print(".3f")
        print(".3f")

        # Get range category
        category = detector.get_current_range_category()
        print(f"📍 Range Category: {category}")

        # Perform averaged measurement for better accuracy
        print("\n📏 Performing averaged distance measurement (10 samples)...")
        avg_measurement = detector.measure_distance_averaged(10)

        print("📊 Averaged Measurement Results:")
        print(".2f")
        print(".3f")
        print(".3f")
        print(".3f")

    except Exception as e:
        print(f"❌ Range detection error: {e}")
        return False

    return True


def demo_laser_communication():
    """Demonstrate laser engine setup and alignment."""
    print("\n🔴 RealGibber Laser Communication Demo")
    print("=" * 50)

    try:
        # Initialize laser engine for visible light communication
        laser = LaserEngine(
            laser_type="Visible",
            modulation_scheme="OOK",
            max_power_mw=50.0,
            range_meters=100.0
        )
        print("🔴 Initializing laser engine...")

        laser.initialize()
        print("✅ Laser engine initialized successfully")

        # Check alignment status
        alignment = laser.get_alignment_status()
        print("🎯 Alignment Status:")
        print(f"   Aligned: {alignment.is_aligned}")
        print(".2f")
        print(".2f")

        # Enable adaptive mode with range detector integration
        range_detector = RangeDetector()
        range_detector.initialize()

        print("🔄 Enabling adaptive power control...")
        laser.enable_adaptive_mode(range_detector)
        print("✅ Adaptive mode enabled")

        # Simulate data transmission
        test_data = b"Hello, RealGibber! This is a test message for laser communication."
        print(f"\n📤 Transmitting {len(test_data)} bytes via laser...")

        laser.transmit_data(test_data)
        print("✅ Data transmission completed")

    except Exception as e:
        print(f"❌ Laser communication error: {e}")
        return False

    return True


def demo_weather_integration():
    """Demonstrate weather-aware mission planning."""
    print("\n🌤️ RealGibber Weather Integration Demo")
    print("=" * 50)

    try:
        # Initialize weather manager
        weather_mgr = WeatherManager(max_stations=50)
        print("🌤️ Weather manager initialized")

        # Create sample weather data
        location = GeoCoordinate(
            latitude=45.5231,  # Seattle coordinates
            longitude=-122.6765,
            altitude_msl=0.0
        )

        weather_data = WeatherData(
            timestamp=time.time(),
            location=location,
            temperature_celsius=18.5,
            humidity_percent=72.0,
            wind_speed_mps=3.2,
            wind_direction_degrees=225.0,
            gust_speed_mps=5.1,
            visibility_meters=12000.0,
            precipitation_rate_mmh=0.0,
            pressure_hpa=1015.2,
            cloud_cover_percent=45.0,
            lightning_probability=15.0
        )

        # Update weather conditions
        weather_mgr.update_weather(weather_data)
        print("✅ Weather data updated")

        # Assess weather impact on drone operations
        from realgibber import DroneSpecifications, MissionPayload

        drone_specs = DroneSpecifications(
            max_wind_speed_mps=12.0,
            max_speed_mps=15.0,
            abort_gust_threshold_mps=8.0,
            power_wind_coefficient=0.15,
            mass_kg=2.5,
            battery_capacity_wh=1200.0,
            sensor_count=3
        )

        mission = MissionPayload("Weather Test Mission", [0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF])

        # Assess weather impact
        impact = weather_mgr.assess_weather_impact(mission, drone_specs)
        print("🌤️ Weather Impact Assessment:")
        print(".2f")
        print(f"   Wind Impact: {impact.wind_impact.track_deviation_degrees:.1f}° deviation")
        print(".1f")
        print(".1f")
        print(f"   Recommended Actions: {len(impact.recommended_actions)} actions")

        # Validate mission constraints
        validation = weather_mgr.validate_mission_constraints(mission, drone_specs)
        print(f"\n✅ Mission Valid: {validation.is_valid}")
        if not validation.is_valid:
            print(f"   Violations: {len(validation.violations)}")
            for violation in validation.violations:
                print(f"   - {violation.description}")

    except Exception as e:
        print(f"❌ Weather integration error: {e}")
        return False

    return True


def demo_performance_monitoring():
    """Demonstrate performance monitoring and benchmarking."""
    print("\n📊 RealGibber Performance Monitoring Demo")
    print("=" * 50)

    try:
        # Initialize performance monitor
        monitor = PerformanceMonitor(max_history=100)
        print("📊 Performance monitor initialized")

        # Run benchmark suite
        print("🏃 Running performance benchmark suite (5 seconds)...")
        benchmarks = monitor.run_benchmark_suite(5)

        print("📈 Benchmark Results:")
        for i, benchmark in enumerate(benchmarks):
            print(f"   Benchmark {i+1}: {benchmark.benchmark_type}")
            print(".2f")
            print(".2f")

        # Get current performance metrics
        current_metrics = monitor.get_current_metrics()
        if current_metrics:
            metrics = current_metrics
            print("\n📊 Current Performance Metrics:")
            print(".2f")
            print(".2f")
            print(".1f")

    except Exception as e:
        print(f"❌ Performance monitoring error: {e}")
        return False

    return True


def demo_security_features():
    """Demonstrate security management capabilities."""
    print("\n🔒 RealGibber Security Features Demo")
    print("=" * 50)

    try:
        # Initialize security manager with high security level
        security = SecurityManager("High")
        print("🔒 Security manager initialized (High level)")

        # Test PIN validation
        print("🔐 Testing PIN validation...")
        try:
            security.validate_pin("1234")  # This should work
            print("✅ PIN validation successful")
        except Exception as e:
            print(f"❌ PIN validation failed: {e}")

        # Test permission checking
        print("🛡️ Testing permission system...")
        try:
            security.check_permission("Read", "Local")
            print("✅ Read permission granted")
        except Exception as e:
            print(f"❌ Permission check failed: {e}")

        try:
            security.check_permission("Execute", "Network")
            print("✅ Network execute permission granted")
        except Exception as e:
            print(f"❌ Network execute permission denied: {e}")

    except Exception as e:
        print(f"❌ Security features error: {e}")
        return False

    return True


def main():
    """Main demo function."""
    print("🚀 RealGibber Python Bindings Comprehensive Demo")
    print("=" * 60)
    print("This demo showcases the complete RealGibber Python API")
    print("including range detection, laser communication, weather integration,")
    print("performance monitoring, and security features.\n")

    # Run all demonstrations
    demos = [
        ("Range Detection", demo_range_detection),
        ("Laser Communication", demo_laser_communication),
        ("Weather Integration", demo_weather_integration),
        ("Performance Monitoring", demo_performance_monitoring),
        ("Security Features", demo_security_features),
    ]

    results = []
    for name, demo_func in demos:
        print(f"\n{'='*20} {name} {'='*20}")
        success = demo_func()
        results.append((name, success))
        time.sleep(1)  # Brief pause between demos

    # Summary
    print("\n" + "=" * 60)
    print("🎯 Demo Summary:")
    successful = 0
    for name, success in results:
        status = "✅ PASSED" if success else "❌ FAILED"
        print(f"   {name}: {status}")
        if success:
            successful += 1

    print(f"\n📊 Results: {successful}/{len(results)} demos passed")

    if successful == len(results):
        print("🎉 All demonstrations completed successfully!")
        print("RealGibber Python bindings are ready for production use.")
    else:
        print("⚠️ Some demonstrations failed. Check hardware availability and configuration.")

    print("\n🔗 For more information, visit: https://github.com/your-org/realgibber")


if __name__ == "__main__":
    main()