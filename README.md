# Short-range-real-Gibberlink
# Real-Gibberlink — short-range secure handshake (audio + visual)

# Real-GibberLink: Secure Directional Communication Protocol
![Marketing illustration](1_reflections/Marketing%20illustratiV1.1.png)
### Purpose
Expose the limits of theatrical "AI sound languages" and provide a robust, verifiable alternative with ultra-short-range pairing (100-300ms) and long-range directional communication (10-200m) using coupled audio-visual-laser channels for enhanced security and reliability.
Contactless, no mechanical wear


Use Case :
Access Badge
Use Case_ Secure Mission Transfer for Autonomous Drones
Use Cases  EVs, Drones, robots and Autonomous Charging Systems, Warehouse, Confidential zones, limited RF Zones 

....

### Version
**Rgibberlink-core v0.3.0** - Currently focused on short-range protocols with long-range architecture designed. License: MIT (GPL/AGPL pending migration).

## Architecture

RGibberLink implements secure, short-range pairing protocols and long-range directional communication systems:

### Short-Range Protocol (Ultra-Short Range, 0-5m)
- **Ultrasonic Burst**: FSK-modulated audio (18-22 kHz) for synchronization and anti-replay protection
- **High-Density Visual Codes**: QR codes with CBOR compression and Reed-Solomon ECC
- **Validation Time**: 100-300 ms end-to-end handshake
- **Security**: ECDH key exchange + AES-GCM + anti-replay tokens

### Long-Range Extension (10-200m Direct Line-of-Sight)
- **Laser Channel**: Modulated visible/IR laser with OOK/PWM/QM modulation for high-bandwidth data
- **Focused Ultrasound Beam**: Parametric audio for directional control and synchronization
- **Coupled Validation**: Requires simultaneous receipt on multiple directional channels
- **Adaptive ECC**: Convolutional + Reed-Solomon codes adapting to atmospheric conditions

### Core Components
- **Rust Core Library** (`rgibberlink-core`): Crypto engines, protocol state machine, modulation handlers
- **Android App** (Kotlin): Camera/microphone/laser integration with JNI C++ bridge
- **Python Bench Tools**: Latency, BER, ECC effectiveness testing (pytest-based)
- **Cross-Platform Support**: Linux/Android/Windows with fallback mechanisms

### Key Features
- **Multi-Channel Coupling**: Enhanced security through correlated channel validation
- **Adaptive Error Correction**: Weather/environmental compensation for long-range
- **Range Detection**: Ultrasonic ranging for power/parameter optimization
- **Fallback Management**: Automatic degradation from long-range to short-range modes
- **Security Manager**: Permission-based access with peer trust assessment
- **Signed Logging**: Tamper-evident session logs with Ed25519 signatures

## Installation

### Dependencies
- **Rust**: `cargo` (1.70+)
- **Python**: 3.8+ with `pip` for benchmarking
- **Android SDK/NDK**: For mobile app (Android Studio)

### Building the Core Library
```bash
cd rgibberlink-core
cargo build --release                    # Release build
cargo build --release --features python  # With Python bindings
```

### Python Bench Tools
```bash
cd tests/
python -m venv .venv
source .venv/bin/activate  # On Windows: .venv\Scripts\activate
pip install -r requirements.txt
```

### Android App
```bash
# In Android Studio
File > Open > Rgibberlink/android-app/
# Build and run on device with camera/microphone permissions
```

### Testing
Run the complete test suite:
```bash
python tests/run_tests.py  # Automated test runner
# Or individual categories:
pytest tests/ -m unit         # Unit tests
pytest tests/ -m integration  # Integration tests
pytest tests/ -m performance  # Performance benchmarks
pytest tests/ -m robustness   # Robustness tests
pytest tests/ -m security     # Security tests
```

## Performance Targets
- **Complete Handshake**: <300ms (target: 100-300ms)
- **QR Code Generation**: <10ms
- **Crypto Operations**: <20ms (key gen + encrypt/decrypt)
- **Concurrent Operations**: Multiple handshakes supported
- **Memory Overhead**: Minimal (~28 bytes per message)
- **Battery Life**: Optimized for mobile usage

### Safety & ethics
This project critiques ideas with evidence. It avoids personal attacks or defamation. Please keep discussion professional and data-driven.

### License
GPL/AGPL


