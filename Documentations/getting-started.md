# Getting Started with RealGibber

## Welcome to RealGibber

RealGibber is a comprehensive suite of secure directional communication protocols designed specifically for mission-critical autonomous systems. This guide will help you get started quickly with your first RealGibber implementation, from installation to your first secure communication.

## Prerequisites

Before you begin, ensure you have the following installed:

### System Requirements
- **Operating System**: Windows 10+, macOS 11+, or Ubuntu 18.04+
- **Rust**: Version 1.70.0 or higher
- **Python**: Version 3.8+ (optional, for Python bindings)
- **Android SDK**: Version 11+ (for Android development)

### Hardware Requirements (for full functionality)
- **Camera**: With autofocus capability (1080p minimum for QR code scanning)
- **Microphone/Speaker**: Low-latency audio I/O for ultrasonic communication
- **GPS Module**: Optional, for location-based features
- **Laser Module**: Required for long-range communication (50-200m range)

## Quick Installation

### 1. Clone the Repository

```bash
git clone https://github.com/your-org/realgibber.git
cd realgibber
```

### 2. Install Rust Dependencies

```bash
# Install Rust if not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install required targets for Android development (optional)
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
```

### 3. Build the Core Library

```bash
# Build the Rust core library
cd rgibberlink-core
cargo build --release

# Run tests to verify installation
cargo test
```

### 4. Python Bindings (Optional)

```bash
# Install maturin for Python bindings
pip install maturin

# Build and install Python bindings
maturin develop --release

# Verify Python installation
python -c "import gibberlink_core; print('RealGibber Python bindings installed successfully')"
```

## Your First RealGibber Application

### Basic Rust Example

Create a new file `hello_real_gibber.rs`:

```rust
use gibberlink_core::RgibberLink;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Initializing RealGibber...");

    // Initialize the RealGibber communication system
    let mut link = RgibberLink::new();

    // Start a short-range handshake
    link.initiate_handshake().await?;

    // Prepare a message
    let message = b"Hello, RealGibber World!";

    // Encrypt and prepare for transmission
    let encrypted = link.encrypt_message(message).await?;

    println!("Message encrypted and ready for transmission: {} bytes", encrypted.len());

    Ok(())
}
```

Run it with:

```bash
rustc hello_real_gibber.rs --extern gibberlink_core=target/release/libgibberlink_core.rlib
./hello_real_gibber
```

### Python Example

Create `hello_real_gibber.py`:

```python
import asyncio
from gibberlink_core import RgibberLink, WeatherManager

async def main():
    print("Initializing RealGibber...")

    # Initialize communication link
    link = RgibberLink()

    # Initialize weather manager for environmental awareness
    weather_mgr = WeatherManager(100)  # Cache size

    print("RealGibber initialized successfully!")

    # Example: Assess weather conditions
    # Note: In a real application, you'd provide actual coordinates and drone specs
    print("Weather assessment example (requires real coordinates)")

if __name__ == "__main__":
    asyncio.run(main())
```

Run it with:

```bash
python hello_real_gibber.py
```

## Next Steps

### 🔰 Beginner Topics

1. **[Understanding Communication Modes](README.md#communication-modes)**
   - Learn about short-range (QR + ultrasound) vs long-range (laser + ultrasound) communication

2. **[Security Fundamentals](technical/security.md)**
   - Understand directional security and cryptographic features

3. **[API Basics](api/api-python.md)**
   - Explore the core APIs for Rust and Python

### 🔧 Development Setup

1. **[Android Development](ANDROID_INTEGRATION_GUIDE.md)**
   - Set up Android development environment
   - Deploy to mobile devices

2. **[Testing](CONTRIBUTING.md#testing)**
   - Learn about the comprehensive test suite
   - Run integration tests

### 🚀 Advanced Topics

1. **[Mission Control](README.md#api-overview)**
   - Implement formation flight and complex missions

2. **[Performance Tuning](DEPLOYMENT_GUIDE.md#performance-tuning)**
   - Optimize for your specific use case

3. **[Security Hardening](technical/security.md)**
   - Implement advanced security measures

## Troubleshooting

### Common Issues

**Build fails with "linker not found"**
```bash
# On Ubuntu/Debian
sudo apt-get install build-essential

# On macOS
xcode-select --install

# On Windows
# Install Visual Studio Build Tools with C++ support
```

**Python bindings won't install**
```bash
# Ensure you have the right Python version
python --version  # Should be 3.8+

# Try installing in development mode
pip install -e .
```

**Android build fails**
```bash
# Ensure Android SDK and NDK are properly configured
export ANDROID_HOME=/path/to/android/sdk
export ANDROID_NDK_HOME=$ANDROID_HOME/ndk/25.2.9519653
```

### Getting Help

- **Documentation**: Check the [full documentation](README.md) for detailed guides
- **Examples**: Look at the `examples/` directory for working code samples
- **Issues**: Report bugs on [GitHub Issues](https://github.com/your-org/realgibber/issues)
- **Community**: Join our [Discord community](https://discord.gg/realgibber) for support

## What's Next?

Now that you have RealGibber up and running, you can:

1. **Explore the examples** in the `examples/` directory
2. **Read the API documentation** for your preferred language
3. **Set up Android development** if you need mobile deployment
4. **Learn about mission control** for complex autonomous operations

Welcome to the RealGibber community! 🎉