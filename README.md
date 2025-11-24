# short-range-real-gibberlink
# gibberlink — short-range secure handshake (audio + visual)

## Purpose
Expose the limits of theatrical "AI sound languages" and provide a robust, verifiable alternative:
a short-range protocol combining an ultrasonic burst (sync + anti-replay) with a high-density visual
code (payload + ECC), achieving end-to-end validation in 100–300 ms.

## Scope
- Rust core for crypto, audio modulation, visual code generation/decoding.
- Android app (Kotlin) for camera/microphone integration.
- Python bench tools for latency, BER, ECC effectiveness.

## Safety & ethics
This project critiques ideas with evidence. It avoids personal attacks or defamation.
Please keep discussion professional and data-driven.

## Quick start
- Core: `cargo build -p gibberlink-core`
- Bench: `python -m venv .venv && source .venv/bin/activate && pip install -r bench/requirements.txt`
- Android: open `mobile/android/` in Android Studio, run on-device.

## License
GPL/AGPL

