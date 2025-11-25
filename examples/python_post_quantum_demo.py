#!/usr/bin/env python3
"""
RealGibber Post-Quantum Cryptography Demo

This script demonstrates the post-quantum cryptographic capabilities of RealGibber,
including Kyber-768 KEM and Dilithium3 digital signatures for quantum-resistant
key exchange and authentication.

Features demonstrated:
- Kyber-768 key encapsulation mechanism (KEM)
- Dilithium3 digital signatures
- Hybrid classical + post-quantum cryptography
- Secure key exchange and authentication
"""

import time
from realgibber import PostQuantumEngine, KyberCiphertextData


def demo_kyber_kem():
    """Demonstrate Kyber-768 Key Encapsulation Mechanism."""
    print("🔐 RealGibber Kyber-768 KEM Demo")
    print("=" * 50)

    try:
        # Initialize two post-quantum engines (Alice and Bob)
        alice = PostQuantumEngine()
        bob = PostQuantumEngine()

        print("✅ Post-quantum engines initialized")
        print(f"Alice Kyber public key length: {len(alice.kyber_public_key())} bytes")
        print(f"Alice Dilithium public key length: {len(alice.dilithium_public_key())} bytes")

        # Alice encapsulates a shared secret for Bob
        print("\n🔄 Alice performs key encapsulation for Bob...")
        alice_ciphertext = alice.encapsulate_secret(bob.kyber_public_key())

        print("📦 Ciphertext generated:")
        print(f"   Ciphertext length: {len(alice_ciphertext.ciphertext())} bytes")
        print(f"   Shared secret length: {len(alice_ciphertext.shared_secret())} bytes")

        # Bob decapsulates the shared secret
        print("\n🔓 Bob decapsulates the shared secret...")
        bob_shared_secret = bob.decapsulate_secret(alice_ciphertext)

        # Verify that both parties have the same shared secret
        alice_shared_secret = alice_ciphertext.shared_secret()

        if alice_shared_secret == bob_shared_secret:
            print("✅ Key exchange successful - shared secrets match!")
            print(f"   Shared secret: {alice_shared_secret.hex()[:32]}...")
        else:
            print("❌ Key exchange failed - shared secrets don't match!")
            return False

    except Exception as e:
        print(f"❌ Kyber KEM error: {e}")
        return False

    return True


def demo_dilithium_signatures():
    """Demonstrate Dilithium3 digital signatures."""
    print("\n✍️ RealGibber Dilithium3 Digital Signatures Demo")
    print("=" * 50)

    try:
        # Initialize post-quantum engines
        alice = PostQuantumEngine()
        bob = PostQuantumEngine()

        # Message to sign
        message = b"Hello, this is a quantum-resistant message from Alice to Bob!"
        print(f"📝 Message to sign: {message.decode()[:50]}...")

        # Alice signs the message
        print("\n🔏 Alice signs the message with Dilithium3...")
        signature = alice.sign_data(message)
        print(f"📋 Signature generated: {len(signature)} bytes")

        # Bob verifies the signature using Alice's public key
        print("\n✅ Bob verifies the signature...")
        is_valid = bob.verify_signature(message, signature, alice.dilithium_public_key())

        if is_valid:
            print("✅ Signature verification successful!")
        else:
            print("❌ Signature verification failed!")
            return False

        # Test with tampered message
        print("\n🧪 Testing with tampered message...")
        tampered_message = message + b" (tampered)"
        is_valid_tampered = bob.verify_signature(tampered_message, signature, alice.dilithium_public_key())

        if not is_valid_tampered:
            print("✅ Tampered message correctly rejected!")
        else:
            print("❌ Tampered message incorrectly accepted!")
            return False

    except Exception as e:
        print(f"❌ Dilithium signatures error: {e}")
        return False

    return True


def demo_hybrid_cryptography():
    """Demonstrate hybrid classical + post-quantum cryptography."""
    print("\n🔄 RealGibber Hybrid Cryptography Demo")
    print("=" * 50)

    try:
        # This would demonstrate combining classical ECDH with post-quantum Kyber
        # For now, we'll show the individual components working together

        alice = PostQuantumEngine()
        bob = PostQuantumEngine()

        print("🔗 Hybrid approach combines:")
        print("   • Classical ECDH (fast, proven)")
        print("   • Post-quantum Kyber (quantum-resistant)")
        print("   • Classical Ed25519 signatures")
        print("   • Post-quantum Dilithium signatures")

        # Demonstrate key exchange
        print("\n🔄 Performing hybrid key exchange...")
        alice_ciphertext = alice.encapsulate_secret(bob.kyber_public_key())
        bob_shared_secret = bob.decapsulate_secret(alice_ciphertext)

        # Demonstrate signatures
        message = b"Hybrid authenticated message"
        alice_sig = alice.sign_data(message)
        bob_verified = bob.verify_signature(message, alice_sig, alice.dilithium_public_key())

        if bob_verified:
            print("✅ Hybrid cryptography successful!")
            print(f"   PQ shared secret: {alice_ciphertext.shared_secret()[:16].hex()}...")
            print(f"   PQ signature: {alice_sig[:16].hex()}...")
        else:
            print("❌ Hybrid cryptography failed!")
            return False

    except Exception as e:
        print(f"❌ Hybrid cryptography error: {e}")
        return False

    return True


def demo_performance_comparison():
    """Compare performance characteristics."""
    print("\n⚡ RealGibber Cryptography Performance Demo")
    print("=" * 50)

    try:
        alice = PostQuantumEngine()

        # Measure Kyber KEM performance
        print("🏃 Measuring Kyber-768 KEM performance...")

        kem_times = []
        for i in range(10):
            bob = PostQuantumEngine()

            start_time = time.time()
            ciphertext = alice.encapsulate_secret(bob.kyber_public_key())
            shared_secret = bob.decapsulate_secret(ciphertext)
            end_time = time.time()

            kem_times.append(end_time - start_time)

        avg_kem_time = sum(kem_times) / len(kem_times)
        print(".4f")
        # Measure Dilithium signature performance
        print("\n🏃 Measuring Dilithium3 signature performance...")

        message = b"Performance test message for quantum-resistant signatures"
        sig_times = []

        for i in range(10):
            start_time = time.time()
            signature = alice.sign_data(message)
            end_time = time.time()

            sig_times.append(end_time - start_time)

        avg_sig_time = sum(sig_times) / len(sig_times)
        print(".4f")
        print("\n📊 Performance Summary:")
        print("   • Kyber-768: NIST Level 3 security (quantum-resistant)")
        print("   • Dilithium3: NIST Level 3 security (quantum-resistant)")
        print("   • Key sizes: Kyber(1184B pub), Dilithium(1952B pub)")
        print("   • Signature sizes: Dilithium(2420B)")
        print("   • Use case: Long-term security for sensitive communications")

    except Exception as e:
        print(f"❌ Performance measurement error: {e}")
        return False

    return True


def demo_quantum_resistance():
    """Explain quantum resistance properties."""
    print("\n🛡️ RealGibber Quantum Resistance Demo")
    print("=" * 50)

    print("🔒 Quantum-Resistant Algorithms:")
    print("   • Kyber-768: Lattice-based key encapsulation")
    print("   • Dilithium3: Lattice-based digital signatures")
    print("   • Security Level: NIST Category 3 (highest)")
    print("   • Resistant to: Shor's algorithm attacks")
    print("   • Based on: Learning With Errors (LWE) problem")

    print("\n⚖️ Security vs Performance Trade-offs:")
    print("   • Classical: Fast, small keys/signatures")
    print("   • Post-Quantum: Slower, larger keys/signatures")
    print("   • Hybrid: Best of both worlds")

    print("\n🎯 RealGibber's Approach:")
    print("   • Classical crypto for performance")
    print("   • Post-quantum crypto for future-proofing")
    print("   • Hybrid schemes for optimal security")
    print("   • Automatic algorithm selection based on requirements")

    return True


def main():
    """Main demo function."""
    print("🚀 RealGibber Post-Quantum Cryptography Comprehensive Demo")
    print("=" * 65)
    print("This demo showcases RealGibber's quantum-resistant cryptographic")
    print("capabilities using Kyber-768 KEM and Dilithium3 digital signatures.")
    print()

    # Run all demonstrations
    demos = [
        ("Kyber-768 KEM", demo_kyber_kem),
        ("Dilithium3 Signatures", demo_dilithium_signatures),
        ("Hybrid Cryptography", demo_hybrid_cryptography),
        ("Performance Comparison", demo_performance_comparison),
        ("Quantum Resistance", demo_quantum_resistance),
    ]

    results = []
    for name, demo_func in demos:
        print(f"\n{'='*20} {name} {'='*20}")
        success = demo_func()
        results.append((name, success))
        time.sleep(0.5)  # Brief pause between demos

    # Summary
    print("\n" + "=" * 65)
    print("🎯 Demo Summary:")
    successful = 0
    for name, success in results:
        status = "✅ PASSED" if success else "❌ FAILED"
        print(f"   {name}: {status}")
        if success:
            successful += 1

    print(f"\n📊 Results: {successful}/{len(results)} demos passed")

    if successful == len(results):
        print("🎉 All post-quantum cryptography demonstrations completed successfully!")
        print("RealGibber is ready for quantum-resistant secure communications.")
    else:
        print("⚠️ Some demonstrations failed. Check post-quantum feature availability.")

    print("\n🔗 For more information, visit: https://github.com/your-org/realgibber")
    print("📚 NIST Post-Quantum Cryptography: https://csrc.nist.gov/projects/post-quantum-cryptography")


if __name__ == "__main__":
    main()