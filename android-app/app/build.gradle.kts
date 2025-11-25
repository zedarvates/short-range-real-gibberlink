plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "com.Rgibberlink"
    compileSdk = 35

    defaultConfig {
        applicationId = "com.Rgibberlink"
        minSdk = 24
        targetSdk = 35
        versionCode = 3
        versionName = "0.3.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"

        externalNativeBuild {
            cmake {
                cppFlags("-std=c++17", "-frtti", "-fexceptions")
                abiFilters("arm64-v8a")
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }

    externalNativeBuild {
        cmake {
            path = file("src/main/cpp/CMakeLists.txt")
        }
    }

    buildFeatures {
        viewBinding = true
    }
}

dependencies {
    implementation("androidx.core:core-ktx:1.12.0")
    implementation("androidx.appcompat:appcompat:1.6.1")
    implementation("com.google.android.material:material:1.10.0")
    implementation("androidx.constraintlayout:constraintlayout:2.1.4")

    // Camera and QR scanning with long-range support
    implementation("androidx.camera:camera-camera2:1.3.1")
    implementation("androidx.camera:camera-lifecycle:1.3.1")
    implementation("androidx.camera:camera-view:1.3.1")
    implementation("com.google.mlkit:barcode-scanning:17.2.0")

    // Face detection for human validation
    implementation("com.google.mlkit:face-detection:16.1.5")

    // Enhanced audio processing for parametric audio and ultrasonic beams
    implementation("androidx.media:media:1.6.0")
    implementation("androidx.media2:media2-session:1.2.1")

    // Hardware acceleration and signal processing
    implementation("androidx.concurrent:concurrent-futures:1.1.0")

    // Location services for range detection
    implementation("com.google.android.gms:play-services-location:21.0.1")

    // Work manager for background hardware monitoring
    implementation("androidx.work:work-runtime-ktx:2.9.0")

    // Lifecycle components for hardware state management
    implementation("androidx.lifecycle:lifecycle-livedata-ktx:2.7.0")
    implementation("androidx.lifecycle:lifecycle-viewmodel-ktx:2.7.0")

    // Security and cryptography for hardware-backed keys
    implementation("androidx.security:security-crypto:1.1.0-alpha06")
    implementation("androidx.biometric:biometric:1.1.0")

    // Long-range hardware specific dependencies
    implementation("androidx.core:core-splashscreen:1.0.1")

    // Testing dependencies
    testImplementation("junit:junit:4.13.2")
    testImplementation("org.mockito:mockito-core:5.8.0")
    testImplementation("androidx.arch.core:core-testing:2.2.0")

    androidTestImplementation("androidx.test.ext:junit:1.1.5")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.5.1")
    androidTestImplementation("androidx.test:rules:1.5.0")

    // Signal Protocol library for end-to-end encryption
    implementation("org.whispersystems:signal-protocol-java:2.8.1")

    // Native library dependencies for long-range hardware
    implementation(fileTree(mapOf("dir" to "libs", "include" to listOf("*.jar", "*.aar"))))
}
