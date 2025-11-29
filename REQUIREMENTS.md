# RealGibberLink Requirements

This document outlines the system requirements, hardware specifications, software dependencies, and prerequisites for building and running RealGibberLink, a secure air-gapped short-range communication system for robots and drones.

## System Requirements

### Minimum System Requirements
- **Operating System**: Android 11 (API 30) or higher for mobile deployment; Linux/Windows/macOS for development
- **CPU**: Quad-core processor minimum, octa-core recommended
- **RAM**: 4GB minimum, 8GB recommended
- **Storage**: 2GB available space
- **Network**: LTE/5G with satellite fallback support (optional for extended operations)

### Development Environment Requirements
- **Android Development**: Android Studio Arctic Fox or later, JDK 11+, Android SDK API 30+
- **Rust Development**: Rust 1.70.0 or higher with Cargo
- **Python Development**: Python 3.8+ with pip for bindings and examples

## Hardware Specifications

### Core Hardware Components
- **Camera**: 1080p minimum resolution with autofocus capability
- **Audio System**: Low-latency microphone/speaker for ultrasonic communication (18-22kHz range)
- **Laser Module**: 10-50mW output power, 405-980nm wavelength for directional communication
- **GPS Receiver**: 5m accuracy with differential GPS support
- **IMU (Inertial Measurement Unit)**: 9-axis sensor for orientation and motion tracking
- **Communication Radios**: LTE/5G modem with satellite backup (optional)

### Special Hardware for Secure Directional Communication
- **Optical/IR LEDs**: For visual signaling and QR code projection
- **Directional Antennas**: For focused ultrasonic transmission
- **Environmental Sensors**: Temperature, humidity, and wind speed sensors for adaptive communication
- **Hardware Security Module (HSM)**: For secure key storage and cryptographic operations (recommended for production deployments)

## Software Dependencies

### Core Dependencies
- **Android NDK**: r25b or higher for native C++/Rust code compilation
- **Rust Toolchain**: Including rustc, cargo, and target architectures (ARM64, x86_64, ARMv7)
- **Python Libraries**: PyO3 for Rust-Python bindings, cryptography libraries for demos
- **WebAssembly Tools**: For browser-based deployment (optional)

### Build Tools
- **Gradle**: 7.0+ for Android project builds
- **CMake**: For native code compilation in Android NDK
- **Cargo**: Rust package manager and build tool

### Runtime Dependencies
- **Android Runtime**: ART with native library support
- **OpenSSL/LibreSSL**: For cryptographic operations
- **CBOR Libraries**: For efficient binary data serialization

## Prerequisites for Building and Running

### Android Development Setup
1. Install Android Studio with Android SDK and NDK
2. Configure JDK 11+ and set JAVA_HOME
3. Enable USB debugging on target Android devices
4. Install required SDK platforms and build tools

### Rust Environment Setup
1. Install Rust via rustup (https://rustup.rs/)
2. Add Android targets: `rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android`
3. Install cargo-ndk for Android cross-compilation

### Python Environment Setup
1. Install Python 3.8+ and pip
2. Install required packages: `pip install pyo3 cryptography numpy`

### Building the Project
1. Clone the repository
2. For Android app: Open `android-app/` in Android Studio and build
3. For Rust core: Run `cargo build --release` in `rgibberlink-core/`
4. For Python examples: Run `python examples/example_script.py`

### Running RealGibberLink
1. Install the Android APK on compatible devices
2. Ensure hardware permissions (camera, microphone, GPS) are granted
3. For development: Use Android emulator with camera/microphone simulation
4. For production: Deploy on devices with required hardware peripherals

## Environmental Requirements
- **Temperature**: -20°C to +50°C operating range
- **Humidity**: 10% to 90% non-condensing
- **Wind Speed**: Up to 15 m/s
- **Visibility**: Minimum 300m for optimal communication
- **Electromagnetic Environment**: Normal industrial conditions (resistant to EMI)

## Additional Notes
- The system is designed for resource-constrained embedded environments
- All cryptographic operations comply with NIST standards
- Multi-platform support includes Android, desktop, and WebAssembly
- For detailed functional requirements, refer to [Documentations/REQUIREMENTS.md](Documentations/REQUIREMENTS.md)

---

*This requirements document is maintained alongside the system implementation. For the latest updates, check the project repository.*