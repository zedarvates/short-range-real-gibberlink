#include <jni.h>
#include <string>
#include <vector>
#include <memory>
#include <mutex>
#include <atomic>
#include <android/log.h>
#include <thread>
#include <condition_variable>

// Include Rust FFI bindings
extern "C" {
    // GibberLink protocol functions
    void* gibberlink_create();
    void gibberlink_destroy(void* ptr);
    bool gibberlink_initiate_handshake(void* ptr);
    int gibberlink_get_state(void* ptr);
    const char* gibberlink_receive_nonce(void* ptr, const uint8_t* nonce, size_t nonce_len);
    bool gibberlink_process_qr_payload(void* ptr, const uint8_t* qr_data, size_t qr_len);
    bool gibberlink_receive_ack(void* ptr);
    uint8_t* gibberlink_encrypt_message(void* ptr, const uint8_t* data, size_t data_len, size_t* out_len);
    uint8_t* gibberlink_decrypt_message(void* ptr, const uint8_t* encrypted_data, size_t encrypted_len, size_t* out_len);
    bool gibberlink_send_audio_data(void* ptr, const uint8_t* data, size_t data_len);
    uint8_t* gibberlink_receive_audio_data(void* ptr, size_t* out_len);
    bool gibberlink_is_receiving(void* ptr);
    const char* gibberlink_generate_qr_code(void* ptr, const uint8_t* payload, size_t payload_len);
    uint8_t* gibberlink_decode_qr_code(void* ptr, const uint8_t* qr_data, size_t qr_len, size_t* out_len);

    // UltrasonicBeamEngine functions
    void* ultrasonic_beam_engine_create();
    void ultrasonic_beam_engine_destroy(void* ptr);
    bool ultrasonic_beam_engine_initialize(void* ptr);
    uint8_t* ultrasonic_beam_engine_generate_parametric_audio(void* ptr, const uint8_t* data, size_t data_len, size_t* out_len);
    bool ultrasonic_beam_engine_transmit_sync_pulse(void* ptr, const uint8_t* pattern, size_t pattern_len);
    bool ultrasonic_beam_engine_transmit_auth_signal(void* ptr, const uint8_t* challenge, size_t challenge_len, const uint8_t* signature, size_t signature_len);
    bool ultrasonic_beam_engine_detect_presence(void* ptr);
    bool ultrasonic_beam_engine_transmit_control_data(void* ptr, const uint8_t* data, size_t data_len, uint8_t priority);
    uint8_t* ultrasonic_beam_engine_receive_beam_signals(void* ptr, size_t* out_len);
    uint8_t* ultrasonic_beam_engine_get_config(void* ptr, size_t* out_len);
    bool ultrasonic_beam_engine_update_config(void* ptr, const uint8_t* config_data, size_t config_len);
    uint8_t* ultrasonic_beam_engine_get_channel_diagnostics(void* ptr, size_t* out_len);
    bool ultrasonic_beam_engine_shutdown(void* ptr);

    // LaserEngine functions
    void* laser_engine_create(const uint8_t* config_data, size_t config_len, const uint8_t* rx_config_data, size_t rx_config_len);
    void laser_engine_destroy(void* ptr);
    bool laser_engine_initialize(void* ptr);
    bool laser_engine_shutdown(void* ptr);
    bool laser_engine_transmit_data(void* ptr, const uint8_t* data, size_t data_len);
    uint8_t* laser_engine_receive_data(void* ptr, uint64_t timeout_ms, size_t* out_len);
    bool laser_engine_set_intensity(void* ptr, float intensity);
    uint8_t* laser_engine_get_alignment_status(void* ptr, size_t* out_len);
    bool laser_engine_set_alignment_target(void* ptr, float x, float y);
    bool laser_engine_auto_align(void* ptr, uint32_t max_attempts);
    uint8_t* laser_engine_get_channel_diagnostics(void* ptr, size_t* out_len);
    bool laser_engine_enable_adaptive_mode(void* ptr);
    bool laser_engine_disable_adaptive_mode(void* ptr);
    bool laser_engine_update_power_profile(void* ptr, const uint8_t* profile_data, size_t profile_len);
    uint8_t* laser_engine_get_current_power_profile(void* ptr, size_t* out_len);
    bool laser_engine_emergency_shutdown(void* ptr);
    uint8_t* laser_engine_get_safety_stats(void* ptr, size_t* out_len);
    bool laser_engine_reset_energy_monitoring(void* ptr);

    // RangeDetector functions
    void* range_detector_create();
    void range_detector_destroy(void* ptr);
    bool range_detector_initialize(void* ptr);
    bool range_detector_is_active(void* ptr);
    bool range_detector_measure_distance(void* ptr, float* out_distance, float* out_strength, float* out_quality);
    bool range_detector_measure_distance_averaged(void* ptr, int samples, float* out_distance, float* out_strength, float* out_quality);
    bool range_detector_measure_distance_fast(void* ptr, float* out_distance, float* out_strength, float* out_quality);
    void range_detector_update_environmental_conditions(void* ptr, float temperature, float humidity, float pressure, float wind_speed, float visibility);
    void range_detector_get_environmental_conditions(void* ptr, float* out_temperature, float* out_humidity, float* out_pressure, float* out_wind_speed, float* out_visibility);
    int range_detector_get_current_range_category(void* ptr);
    int range_detector_get_measurement_history_size(void* ptr);
    bool range_detector_get_measurement_history(void* ptr, int index, float* out_distance, float* out_strength, float* out_quality, long* out_timestamp);
    bool range_detector_shutdown(void* ptr);

    // Hardware capability detection
    uint8_t* detect_hardware_capabilities(size_t* out_len);
    bool check_ultrasonic_hardware_available();
    bool check_laser_hardware_available();
    bool check_photodiode_hardware_available();
    bool check_camera_hardware_available();

    // Memory management for returned data
    void gibberlink_free_data(uint8_t* data);
}

// Logging macros
#define LOG_TAG "RgibberLinkJNI"
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, LOG_TAG, __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, LOG_TAG, __VA_ARGS__)
#define LOGW(...) __android_log_print(ANDROID_LOG_WARN, LOG_TAG, __VA_ARGS__)

// Input validation macros and constants
#define MAX_ARRAY_SIZE (1024 * 1024 * 10) // 10MB max array size
#define MIN_TIMEOUT_MS 0
#define MAX_TIMEOUT_MS 30000 // 30 seconds max timeout
#define MIN_INTENSITY 0.0f
#define MAX_INTENSITY 100.0f
#define MIN_PRIORITY 0
#define MAX_PRIORITY 255

// Validation helper functions
bool validate_pointer(void* ptr, const char* function_name) {
    if (!ptr) {
        LOGE("Null pointer validation failed in %s", function_name);
        return false;
    }
    return true;
}

bool validate_byte_array(JNIEnv* env, jbyteArray array, const char* param_name, const char* function_name) {
    if (!array) {
        LOGE("Null byte array '%s' in %s", param_name, function_name);
        return false;
    }

    jsize len = env->GetArrayLength(array);
    if (len < 0) {
        LOGE("Invalid array length for '%s' in %s: %d", param_name, function_name, len);
        return false;
    }

    if (static_cast<size_t>(len) > MAX_ARRAY_SIZE) {
        LOGE("Array too large for '%s' in %s: %d bytes (max: %d)", param_name, function_name, len, MAX_ARRAY_SIZE);
        return false;
    }

    return true;
}

bool validate_float_array(JNIEnv* env, jfloatArray array, jsize expected_size, const char* param_name, const char* function_name) {
    if (!array) {
        LOGE("Null float array '%s' in %s", param_name, function_name);
        return false;
    }

    jsize len = env->GetArrayLength(array);
    if (len != expected_size) {
        LOGE("Invalid float array size for '%s' in %s: expected %d, got %d", param_name, function_name, expected_size, len);
        return false;
    }

    return true;
}

bool validate_long_array(JNIEnv* env, jlongArray array, jsize expected_size, const char* param_name, const char* function_name) {
    if (!array) {
        LOGE("Null long array '%s' in %s", param_name, function_name);
        return false;
    }

    jsize len = env->GetArrayLength(array);
    if (len != expected_size) {
        LOGE("Invalid long array size for '%s' in %s: expected %d, got %d", param_name, function_name, expected_size, len);
        return false;
    }

    return true;
}

bool validate_timeout(jint timeout_ms, const char* function_name) {
    if (timeout_ms < MIN_TIMEOUT_MS || timeout_ms > MAX_TIMEOUT_MS) {
        LOGE("Invalid timeout in %s: %d ms (valid range: %d-%d)", function_name, timeout_ms, MIN_TIMEOUT_MS, MAX_TIMEOUT_MS);
        return false;
    }
    return true;
}

bool validate_intensity(jfloat intensity, const char* function_name) {
    if (intensity < MIN_INTENSITY || intensity > MAX_INTENSITY) {
        LOGE("Invalid intensity in %s: %f (valid range: %f-%f)", function_name, intensity, MIN_INTENSITY, MAX_INTENSITY);
        return false;
    }
    return true;
}

bool validate_priority(jint priority, const char* function_name) {
    if (priority < MIN_PRIORITY || priority > MAX_PRIORITY) {
        LOGE("Invalid priority in %s: %d (valid range: %d-%d)", function_name, priority, MIN_PRIORITY, MAX_PRIORITY);
        return false;
    }
    return true;
}

bool validate_samples(jint samples, const char* function_name) {
    if (samples <= 0 || samples > 1000) {
        LOGE("Invalid samples count in %s: %d (valid range: 1-1000)", function_name, samples);
        return false;
    }
    return true;
}

bool validate_max_attempts(jint max_attempts, const char* function_name) {
    if (max_attempts <= 0 || max_attempts > 100) {
        LOGE("Invalid max attempts in %s: %d (valid range: 1-100)", function_name, max_attempts);
        return false;
    }
    return true;
}

bool validate_environmental_values(jfloat temperature, jfloat humidity, jfloat pressure,
                                  jfloat wind_speed, jfloat visibility, const char* function_name) {
    if (temperature < -100.0f || temperature > 100.0f) {
        LOGE("Invalid temperature in %s: %f°C", function_name, temperature);
        return false;
    }
    if (humidity < 0.0f || humidity > 100.0f) {
        LOGE("Invalid humidity in %s: %f%%", function_name, humidity);
        return false;
    }
    if (pressure < 800.0f || pressure > 1200.0f) {
        LOGE("Invalid pressure in %s: %f hPa", function_name, pressure);
        return false;
    }
    if (wind_speed < 0.0f || wind_speed > 100.0f) {
        LOGE("Invalid wind speed in %s: %f m/s", function_name, wind_speed);
        return false;
    }
    if (visibility < 0.0f || visibility > 50000.0f) {
        LOGE("Invalid visibility in %s: %f m", function_name, visibility);
        return false;
    }
    return true;
}

bool validate_alignment_coords(jfloat x, jfloat y, const char* function_name) {
    if (x < -1.0f || x > 1.0f || y < -1.0f || y > 1.0f) {
        LOGE("Invalid alignment coordinates in %s: x=%f, y=%f (valid range: -1.0 to 1.0)", function_name, x, y);
        return false;
    }
    return true;
}

// Thread safety helpers with enhanced error handling
class JNIGuard {
private:
    std::mutex& mutex_;
    const char* function_name_;

public:
    bool locked_;

public:
    explicit JNIGuard(std::mutex& mutex, const char* function_name = nullptr)
        : mutex_(mutex), locked_(false), function_name_(function_name) {
        try {
            mutex_.lock();
            locked_ = true;
            if (function_name_) {
                LOGI("Entered critical section for %s", function_name_);
            }
        } catch (const std::system_error& e) {
            LOGE("Failed to acquire mutex lock in %s: %s", function_name_ ? function_name_ : "unknown", e.what());
            locked_ = false;
        }
    }

    ~JNIGuard() {
        if (locked_) {
            try {
                mutex_.unlock();
                if (function_name_) {
                    LOGI("Exited critical section for %s", function_name_);
                }
            } catch (const std::system_error& e) {
                LOGE("Failed to release mutex lock in %s: %s", function_name_ ? function_name_ : "unknown", e.what());
            }
        }
    }

    // Prevent copying
    JNIGuard(const JNIGuard&) = delete;
    JNIGuard& operator=(const JNIGuard&) = delete;
};

// Global mutexes for thread safety
static std::mutex g_protocol_mutex;
static std::mutex g_ultrasonic_mutex;
static std::mutex g_laser_mutex;
static std::mutex g_range_detector_mutex;
static std::mutex g_hardware_mutex;

// Hardware event callback
static std::atomic<jobject> g_callback_object(nullptr);
static JavaVM* g_java_vm = nullptr;

// Safer helper functions with additional validation
jbyteArray create_byte_array(JNIEnv* env, const uint8_t* data, size_t len) {
    if (!env) {
        LOGE("Null JNIEnv in create_byte_array");
        return nullptr;
    }
    if (!data && len > 0) {
        LOGE("Null data with non-zero length in create_byte_array");
        return nullptr;
    }
    if (len == 0) {
        LOGE("Zero length in create_byte_array");
        return nullptr;
    }
    if (len > MAX_ARRAY_SIZE) {
        LOGE("Array size %zu exceeds maximum allowed size %d in create_byte_array", len, MAX_ARRAY_SIZE);
        return nullptr;
    }

    jbyteArray result = env->NewByteArray(len);
    if (!result) {
        LOGE("Failed to create byte array of size %zu in create_byte_array", len);
        return nullptr;
    }

    env->SetByteArrayRegion(result, 0, len, reinterpret_cast<const jbyte*>(data));
    if (env->ExceptionCheck()) {
        LOGE("Exception occurred while setting byte array region in create_byte_array");
        env->ExceptionClear();
        env->DeleteLocalRef(result);
        return nullptr;
    }

    return result;
}

std::vector<uint8_t> get_byte_array_data(JNIEnv* env, jbyteArray array) {
    if (!env) {
        LOGE("Null JNIEnv in get_byte_array_data");
        return {};
    }
    if (!array) {
        LOGE("Null array in get_byte_array_data");
        return {};
    }

    jsize len = env->GetArrayLength(array);
    if (len < 0) {
        LOGE("Negative array length %d in get_byte_array_data", len);
        return {};
    }
    if (len == 0) {
        LOGW("Empty array in get_byte_array_data");
        return {};
    }
    if (static_cast<size_t>(len) > MAX_ARRAY_SIZE) {
        LOGE("Array size %d exceeds maximum allowed size %d in get_byte_array_data", len, MAX_ARRAY_SIZE);
        return {};
    }

    std::vector<uint8_t> result(len);
    env->GetByteArrayRegion(array, 0, len, reinterpret_cast<jbyte*>(result.data()));
    if (env->ExceptionCheck()) {
        LOGE("Exception occurred while getting byte array region in get_byte_array_data");
        env->ExceptionClear();
        return {};
    }

    return result;
}

jstring create_string(JNIEnv* env, const char* str) {
    if (!env) {
        LOGE("Null JNIEnv in create_string");
        return nullptr;
    }
    if (!str) {
        LOGE("Null string in create_string");
        return nullptr;
    }

    jstring result = env->NewStringUTF(str);
    if (!result) {
        LOGE("Failed to create string from '%s' in create_string", str);
        return nullptr;
    }

    if (env->ExceptionCheck()) {
        LOGE("Exception occurred while creating string in create_string");
        env->ExceptionClear();
        return nullptr;
    }

    return result;
}

// JNI function implementations

extern "C" JNIEXPORT jlong JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_createGibberLink(JNIEnv* env, jobject /* this */) {
    JNIGuard guard(g_protocol_mutex);
    try {
        void* ptr = gibberlink_create();
        LOGI("Created GibberLink instance: %p", ptr);
        return reinterpret_cast<jlong>(ptr);
    } catch (const std::exception& e) {
        LOGE("Failed to create GibberLink: %s", e.what());
        return 0;
    }
}

extern "C" JNIEXPORT void JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_destroyGibberLink(JNIEnv* env, jobject /* this */, jlong ptr) {
    JNIGuard guard(g_protocol_mutex);
    if (ptr) {
        gibberlink_destroy(reinterpret_cast<void*>(ptr));
        LOGI("Destroyed GibberLink instance: %p", reinterpret_cast<void*>(ptr));
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_initiateHandshake(JNIEnv* env, jobject /* this */, jlong ptr) {
    JNIGuard guard(g_protocol_mutex);
    if (!ptr) return JNI_FALSE;

    try {
        return gibberlink_initiate_handshake(reinterpret_cast<void*>(ptr)) ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("Handshake initiation failed: %s", e.what());
        return JNI_FALSE;
    }
}

extern "C" JNIEXPORT jint JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_getState(JNIEnv* env, jobject /* this */, jlong ptr) {
    JNIGuard guard(g_protocol_mutex);
    if (!ptr) return 5; // STATE_ERROR

    return gibberlink_get_state(reinterpret_cast<void*>(ptr));
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_receiveNonce(JNIEnv* env, jobject /* this */, jlong ptr, jbyteArray nonce) {
    if (!env) {
        LOGE("Null JNIEnv in receiveNonce");
        return nullptr;
    }

    JNIGuard guard(g_protocol_mutex, "receiveNonce");
    if (!guard.locked_) {
        LOGE("Failed to acquire mutex lock in receiveNonce");
        return nullptr;
    }

    if (!validate_pointer(reinterpret_cast<void*>(ptr), "receiveNonce") ||
        !validate_byte_array(env, nonce, "nonce", "receiveNonce")) {
        return nullptr;
    }

    try {
        auto nonce_data = get_byte_array_data(env, nonce);
        if (nonce_data.empty()) {
            LOGE("Empty nonce data in receiveNonce");
            return nullptr;
        }

        // Validate nonce size (typically 32 bytes for cryptographic nonce)
        if (nonce_data.size() != 32) {
            LOGE("Invalid nonce size %zu in receiveNonce (expected 32)", nonce_data.size());
            return nullptr;
        }

        const char* result = gibberlink_receive_nonce(
            reinterpret_cast<void*>(ptr),
            nonce_data.data(),
            nonce_data.size()
        );

        if (!result) {
            LOGE("gibberlink_receive_nonce returned null");
            return nullptr;
        }

        return create_string(env, result);
    } catch (const std::exception& e) {
        LOGE("Receive nonce failed: %s", e.what());
        return nullptr;
    } catch (...) {
        LOGE("Unknown exception occurred in receiveNonce");
        return nullptr;
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_processQrPayload(JNIEnv* env, jobject /* this */, jlong ptr, jbyteArray qrData) {
    JNIGuard guard(g_protocol_mutex);
    if (!ptr || !qrData) return JNI_FALSE;

    try {
        auto qr_data = get_byte_array_data(env, qrData);
        return gibberlink_process_qr_payload(
            reinterpret_cast<void*>(ptr),
            qr_data.data(),
            qr_data.size()
        ) ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("Process QR payload failed: %s", e.what());
        return JNI_FALSE;
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_receiveAck(JNIEnv* env, jobject /* this */, jlong ptr) {
    JNIGuard guard(g_protocol_mutex);
    if (!ptr) return JNI_FALSE;

    try {
        return gibberlink_receive_ack(reinterpret_cast<void*>(ptr)) ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("Receive ACK failed: %s", e.what());
        return JNI_FALSE;
    }
}

extern "C" JNIEXPORT jbyteArray JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_encryptMessage(JNIEnv* env, jobject /* this */, jlong ptr, jbyteArray data) {
    JNIGuard guard(g_protocol_mutex);
    if (!ptr || !data) return nullptr;

    try {
        auto data_bytes = get_byte_array_data(env, data);
        size_t out_len = 0;
        uint8_t* result = gibberlink_encrypt_message(
            reinterpret_cast<void*>(ptr),
            data_bytes.data(),
            data_bytes.size(),
            &out_len
        );

        if (!result) return nullptr;

        jbyteArray array = create_byte_array(env, result, out_len);
        gibberlink_free_data(result);
        return array;
    } catch (const std::exception& e) {
        LOGE("Encrypt message failed: %s", e.what());
        return nullptr;
    }
}

extern "C" JNIEXPORT jbyteArray JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_decryptMessage(JNIEnv* env, jobject /* this */, jlong ptr, jbyteArray encryptedData) {
    JNIGuard guard(g_protocol_mutex);
    if (!validate_pointer(reinterpret_cast<void*>(ptr), "decryptMessage") ||
        !validate_byte_array(env, encryptedData, "encryptedData", "decryptMessage")) {
        return nullptr;
    }

    try {
        auto encrypted_bytes = get_byte_array_data(env, encryptedData);
        if (encrypted_bytes.empty()) {
            LOGE("Empty encrypted data in decryptMessage");
            return nullptr;
        }

        size_t out_len = 0;
        uint8_t* result = gibberlink_decrypt_message(
            reinterpret_cast<void*>(ptr),
            encrypted_bytes.data(),
            encrypted_bytes.size(),
            &out_len
        );

        if (!result || out_len == 0) return nullptr;

        jbyteArray array = create_byte_array(env, result, out_len);
        gibberlink_free_data(result);
        return array;
    } catch (const std::exception& e) {
        LOGE("Decrypt message failed: %s", e.what());
        return nullptr;
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_sendAudioData(JNIEnv* env, jobject /* this */, jlong ptr, jbyteArray data) {
    JNIGuard guard(g_protocol_mutex);
    if (!validate_pointer(reinterpret_cast<void*>(ptr), "sendAudioData") ||
        !validate_byte_array(env, data, "data", "sendAudioData")) {
        return JNI_FALSE;
    }

    try {
        auto data_bytes = get_byte_array_data(env, data);
        if (data_bytes.empty()) {
            LOGE("Empty audio data in sendAudioData");
            return JNI_FALSE;
        }

        return gibberlink_send_audio_data(
            reinterpret_cast<void*>(ptr),
            data_bytes.data(),
            data_bytes.size()
        ) ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("Send audio data failed: %s", e.what());
        return JNI_FALSE;
    }
}

extern "C" JNIEXPORT jbyteArray JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_receiveAudioData(JNIEnv* env, jobject /* this */, jlong ptr) {
    JNIGuard guard(g_protocol_mutex);
    if (!ptr) return nullptr;

    try {
        size_t out_len = 0;
        uint8_t* result = gibberlink_receive_audio_data(reinterpret_cast<void*>(ptr), &out_len);

        if (!result) return nullptr;

        jbyteArray array = create_byte_array(env, result, out_len);
        gibberlink_free_data(result);
        return array;
    } catch (const std::exception& e) {
        LOGE("Receive audio data failed: %s", e.what());
        return nullptr;
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_isReceiving(JNIEnv* env, jobject /* this */, jlong ptr) {
    JNIGuard guard(g_protocol_mutex);
    if (!validate_pointer(reinterpret_cast<void*>(ptr), "isReceiving")) {
        return JNI_FALSE;
    }

    return gibberlink_is_receiving(reinterpret_cast<void*>(ptr)) ? JNI_TRUE : JNI_FALSE;
}

extern "C" JNIEXPORT jstring JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_generateQrCode(JNIEnv* env, jobject /* this */, jlong ptr, jbyteArray payload) {
    JNIGuard guard(g_protocol_mutex);
    if (!validate_pointer(reinterpret_cast<void*>(ptr), "generateQrCode") ||
        !validate_byte_array(env, payload, "payload", "generateQrCode")) {
        return nullptr;
    }

    try {
        auto payload_data = get_byte_array_data(env, payload);
        if (payload_data.empty()) {
            LOGE("Empty payload data in generateQrCode");
            return nullptr;
        }

        const char* result = gibberlink_generate_qr_code(
            reinterpret_cast<void*>(ptr),
            payload_data.data(),
            payload_data.size()
        );

        return create_string(env, result);
    } catch (const std::exception& e) {
        LOGE("Generate QR code failed: %s", e.what());
        return nullptr;
    }
}

extern "C" JNIEXPORT jbyteArray JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_decodeQrCode(JNIEnv* env, jobject /* this */, jlong ptr, jbyteArray qrData) {
    JNIGuard guard(g_protocol_mutex);
    if (!validate_pointer(reinterpret_cast<void*>(ptr), "decodeQrCode") ||
        !validate_byte_array(env, qrData, "qrData", "decodeQrCode")) {
        return nullptr;
    }

    try {
        auto qr_data = get_byte_array_data(env, qrData);
        if (qr_data.empty()) {
            LOGE("Empty QR data in decodeQrCode");
            return nullptr;
        }

        size_t out_len = 0;
        uint8_t* result = gibberlink_decode_qr_code(
            reinterpret_cast<void*>(ptr),
            qr_data.data(),
            qr_data.size(),
            &out_len
        );

        if (!result || out_len == 0) return nullptr;

        jbyteArray array = create_byte_array(env, result, out_len);
        gibberlink_free_data(result);
        return array;
    } catch (const std::exception& e) {
        LOGE("Decode QR code failed: %s", e.what());
        return nullptr;
    }
}

// UltrasonicBeamEngine JNI implementations

extern "C" JNIEXPORT jlong JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_createUltrasonicBeamEngine(JNIEnv* env, jobject /* this */) {
    JNIGuard guard(g_ultrasonic_mutex);
    try {
        void* ptr = ultrasonic_beam_engine_create();
        LOGI("Created UltrasonicBeamEngine instance: %p", ptr);
        return reinterpret_cast<jlong>(ptr);
    } catch (const std::exception& e) {
        LOGE("Failed to create UltrasonicBeamEngine: %s", e.what());
        return 0;
    }
}

extern "C" JNIEXPORT void JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_destroyUltrasonicBeamEngine(JNIEnv* env, jobject /* this */, jlong ptr) {
    JNIGuard guard(g_ultrasonic_mutex);
    if (ptr) {
        ultrasonic_beam_engine_destroy(reinterpret_cast<void*>(ptr));
        LOGI("Destroyed UltrasonicBeamEngine instance: %p", reinterpret_cast<void*>(ptr));
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_initializeUltrasonicBeamEngine(JNIEnv* env, jobject /* this */, jlong ptr) {
    JNIGuard guard(g_ultrasonic_mutex);
    if (!ptr) return JNI_FALSE;

    try {
        return ultrasonic_beam_engine_initialize(reinterpret_cast<void*>(ptr)) ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("UltrasonicBeamEngine initialization failed: %s", e.what());
        return JNI_FALSE;
    }
}

extern "C" JNIEXPORT jbyteArray JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_generateParametricAudio(JNIEnv* env, jobject /* this */, jlong ptr, jbyteArray data) {
    JNIGuard guard(g_ultrasonic_mutex);
    if (!ptr || !data) return nullptr;

    try {
        auto data_bytes = get_byte_array_data(env, data);
        size_t out_len = 0;
        uint8_t* result = ultrasonic_beam_engine_generate_parametric_audio(
            reinterpret_cast<void*>(ptr),
            data_bytes.data(),
            data_bytes.size(),
            &out_len
        );

        if (!result) return nullptr;

        jbyteArray array = create_byte_array(env, result, out_len);
        gibberlink_free_data(result);
        return array;
    } catch (const std::exception& e) {
        LOGE("Generate parametric audio failed: %s", e.what());
        return nullptr;
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_transmitSyncPulse(JNIEnv* env, jobject /* this */, jlong ptr, jbyteArray pattern) {
    JNIGuard guard(g_ultrasonic_mutex);
    if (!ptr || !pattern) return JNI_FALSE;

    try {
        auto pattern_data = get_byte_array_data(env, pattern);
        return ultrasonic_beam_engine_transmit_sync_pulse(
            reinterpret_cast<void*>(ptr),
            pattern_data.data(),
            pattern_data.size()
        ) ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("Transmit sync pulse failed: %s", e.what());
        return JNI_FALSE;
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_transmitAuthSignal(JNIEnv* env, jobject /* this */, jlong ptr, jbyteArray challenge, jbyteArray signature) {
    JNIGuard guard(g_ultrasonic_mutex);
    if (!ptr || !challenge || !signature) return JNI_FALSE;

    try {
        auto challenge_data = get_byte_array_data(env, challenge);
        auto signature_data = get_byte_array_data(env, signature);
        return ultrasonic_beam_engine_transmit_auth_signal(
            reinterpret_cast<void*>(ptr),
            challenge_data.data(),
            challenge_data.size(),
            signature_data.data(),
            signature_data.size()
        ) ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("Transmit auth signal failed: %s", e.what());
        return JNI_FALSE;
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_detectPresence(JNIEnv* env, jobject /* this */, jlong ptr) {
    JNIGuard guard(g_ultrasonic_mutex);
    if (!ptr) return JNI_FALSE;

    try {
        return ultrasonic_beam_engine_detect_presence(reinterpret_cast<void*>(ptr)) ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("Presence detection failed: %s", e.what());
        return JNI_FALSE;
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_transmitControlData(JNIEnv* env, jobject /* this */, jlong ptr, jbyteArray data, jint priority) {
    JNIGuard guard(g_ultrasonic_mutex);
    if (!ptr || !data) return JNI_FALSE;

    try {
        auto data_bytes = get_byte_array_data(env, data);
        return ultrasonic_beam_engine_transmit_control_data(
            reinterpret_cast<void*>(ptr),
            data_bytes.data(),
            data_bytes.size(),
            static_cast<uint8_t>(priority)
        ) ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("Transmit control data failed: %s", e.what());
        return JNI_FALSE;
    }
}

extern "C" JNIEXPORT jbyteArray JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_receiveBeamSignals(JNIEnv* env, jobject /* this */, jlong ptr) {
    JNIGuard guard(g_ultrasonic_mutex);
    if (!ptr) return nullptr;

    try {
        size_t out_len = 0;
        uint8_t* result = ultrasonic_beam_engine_receive_beam_signals(reinterpret_cast<void*>(ptr), &out_len);

        if (!result) return nullptr;

        jbyteArray array = create_byte_array(env, result, out_len);
        gibberlink_free_data(result);
        return array;
    } catch (const std::exception& e) {
        LOGE("Receive beam signals failed: %s", e.what());
        return nullptr;
    }
}

extern "C" JNIEXPORT jbyteArray JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_getUltrasonicBeamConfig(JNIEnv* env, jobject /* this */, jlong ptr) {
    JNIGuard guard(g_ultrasonic_mutex);
    if (!ptr) return nullptr;

    try {
        size_t out_len = 0;
        uint8_t* result = ultrasonic_beam_engine_get_config(reinterpret_cast<void*>(ptr), &out_len);

        if (!result) return nullptr;

        jbyteArray array = create_byte_array(env, result, out_len);
        gibberlink_free_data(result);
        return array;
    } catch (const std::exception& e) {
        LOGE("Get ultrasonic beam config failed: %s", e.what());
        return nullptr;
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_updateUltrasonicBeamConfig(JNIEnv* env, jobject /* this */, jlong ptr, jbyteArray config) {
    JNIGuard guard(g_ultrasonic_mutex);
    if (!ptr || !config) return JNI_FALSE;

    try {
        auto config_data = get_byte_array_data(env, config);
        return ultrasonic_beam_engine_update_config(
            reinterpret_cast<void*>(ptr),
            config_data.data(),
            config_data.size()
        ) ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("Update ultrasonic beam config failed: %s", e.what());
        return JNI_FALSE;
    }
}

extern "C" JNIEXPORT jbyteArray JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_getUltrasonicChannelDiagnostics(JNIEnv* env, jobject /* this */, jlong ptr) {
    JNIGuard guard(g_ultrasonic_mutex);
    if (!ptr) return nullptr;

    try {
        size_t out_len = 0;
        uint8_t* result = ultrasonic_beam_engine_get_channel_diagnostics(reinterpret_cast<void*>(ptr), &out_len);

        if (!result) return nullptr;

        jbyteArray array = create_byte_array(env, result, out_len);
        gibberlink_free_data(result);
        return array;
    } catch (const std::exception& e) {
        LOGE("Get ultrasonic channel diagnostics failed: %s", e.what());
        return nullptr;
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_shutdownUltrasonicBeamEngine(JNIEnv* env, jobject /* this */, jlong ptr) {
    JNIGuard guard(g_ultrasonic_mutex);
    if (!ptr) return JNI_FALSE;

    try {
        return ultrasonic_beam_engine_shutdown(reinterpret_cast<void*>(ptr)) ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("Shutdown ultrasonic beam engine failed: %s", e.what());
        return JNI_FALSE;
    }
}

// LaserEngine JNI implementations

extern "C" JNIEXPORT jlong JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_createLaserEngine(JNIEnv* env, jobject /* this */, jbyteArray config, jbyteArray rxConfig) {
    JNIGuard guard(g_laser_mutex);
    if (!validate_byte_array(env, config, "config", "createLaserEngine") ||
        !validate_byte_array(env, rxConfig, "rxConfig", "createLaserEngine")) {
        return 0;
    }

    try {
        auto config_data = get_byte_array_data(env, config);
        auto rx_config_data = get_byte_array_data(env, rxConfig);

        if (config_data.empty()) {
            LOGE("Empty config data in createLaserEngine");
            return 0;
        }
        if (rx_config_data.empty()) {
            LOGE("Empty RX config data in createLaserEngine");
            return 0;
        }

        void* ptr = laser_engine_create(
            config_data.data(),
            config_data.size(),
            rx_config_data.data(),
            rx_config_data.size()
        );
        LOGI("Created LaserEngine instance: %p", ptr);
        return reinterpret_cast<jlong>(ptr);
    } catch (const std::exception& e) {
        LOGE("Failed to create LaserEngine: %s", e.what());
        return 0;
    }
}

extern "C" JNIEXPORT void JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_destroyLaserEngine(JNIEnv* env, jobject /* this */, jlong ptr) {
    JNIGuard guard(g_laser_mutex);
    if (ptr) {
        laser_engine_destroy(reinterpret_cast<void*>(ptr));
        LOGI("Destroyed LaserEngine instance: %p", reinterpret_cast<void*>(ptr));
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_initializeLaserEngine(JNIEnv* env, jobject /* this */, jlong ptr) {
    JNIGuard guard(g_laser_mutex);
    if (!ptr) return JNI_FALSE;

    try {
        return laser_engine_initialize(reinterpret_cast<void*>(ptr)) ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("LaserEngine initialization failed: %s", e.what());
        return JNI_FALSE;
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_shutdownLaserEngine(JNIEnv* env, jobject /* this */, jlong ptr) {
    JNIGuard guard(g_laser_mutex);
    if (!ptr) return JNI_FALSE;

    try {
        return laser_engine_shutdown(reinterpret_cast<void*>(ptr)) ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("LaserEngine shutdown failed: %s", e.what());
        return JNI_FALSE;
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_transmitLaserData(JNIEnv* env, jobject /* this */, jlong ptr, jbyteArray data) {
    JNIGuard guard(g_laser_mutex);
    if (!ptr || !data) return JNI_FALSE;

    try {
        auto data_bytes = get_byte_array_data(env, data);
        return laser_engine_transmit_data(
            reinterpret_cast<void*>(ptr),
            data_bytes.data(),
            data_bytes.size()
        ) ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("Transmit laser data failed: %s", e.what());
        return JNI_FALSE;
    }
}

extern "C" JNIEXPORT jbyteArray JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_receiveLaserData(JNIEnv* env, jobject /* this */, jlong ptr, jint timeoutMs) {
    JNIGuard guard(g_laser_mutex);
    if (!validate_pointer(reinterpret_cast<void*>(ptr), "receiveLaserData") ||
        !validate_timeout(timeoutMs, "receiveLaserData")) {
        return nullptr;
    }

    try {
        size_t out_len = 0;
        uint8_t* result = laser_engine_receive_data(
            reinterpret_cast<void*>(ptr),
            static_cast<uint64_t>(timeoutMs),
            &out_len
        );

        if (!result || out_len == 0) return nullptr;

        jbyteArray array = create_byte_array(env, result, out_len);
        gibberlink_free_data(result);
        return array;
    } catch (const std::exception& e) {
        LOGE("Receive laser data failed: %s", e.what());
        return nullptr;
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_setLaserIntensity(JNIEnv* env, jobject /* this */, jlong ptr, jfloat intensity) {
    JNIGuard guard(g_laser_mutex);
    if (!validate_pointer(reinterpret_cast<void*>(ptr), "setLaserIntensity") ||
        !validate_intensity(intensity, "setLaserIntensity")) {
        return JNI_FALSE;
    }

    try {
        return laser_engine_set_intensity(reinterpret_cast<void*>(ptr), intensity) ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("Set laser intensity failed: %s", e.what());
        return JNI_FALSE;
    }
}

extern "C" JNIEXPORT jbyteArray JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_getAlignmentStatus(JNIEnv* env, jobject /* this */, jlong ptr) {
    JNIGuard guard(g_laser_mutex);
    if (!ptr) return nullptr;

    try {
        size_t out_len = 0;
        uint8_t* result = laser_engine_get_alignment_status(reinterpret_cast<void*>(ptr), &out_len);

        if (!result) return nullptr;

        jbyteArray array = create_byte_array(env, result, out_len);
        gibberlink_free_data(result);
        return array;
    } catch (const std::exception& e) {
        LOGE("Get alignment status failed: %s", e.what());
        return nullptr;
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_setAlignmentTarget(JNIEnv* env, jobject /* this */, jlong ptr, jfloat x, jfloat y) {
    JNIGuard guard(g_laser_mutex);
    if (!validate_pointer(reinterpret_cast<void*>(ptr), "setAlignmentTarget") ||
        !validate_alignment_coords(x, y, "setAlignmentTarget")) {
        return JNI_FALSE;
    }

    try {
        return laser_engine_set_alignment_target(reinterpret_cast<void*>(ptr), x, y) ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("Set alignment target failed: %s", e.what());
        return JNI_FALSE;
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_autoAlign(JNIEnv* env, jobject /* this */, jlong ptr, jint maxAttempts) {
    JNIGuard guard(g_laser_mutex);
    if (!validate_pointer(reinterpret_cast<void*>(ptr), "autoAlign") ||
        !validate_max_attempts(maxAttempts, "autoAlign")) {
        return JNI_FALSE;
    }

    try {
        return laser_engine_auto_align(reinterpret_cast<void*>(ptr), maxAttempts) ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("Auto align failed: %s", e.what());
        return JNI_FALSE;
    }
}

extern "C" JNIEXPORT jbyteArray JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_getLaserChannelDiagnostics(JNIEnv* env, jobject /* this */, jlong ptr) {
    JNIGuard guard(g_laser_mutex);
    if (!ptr) return nullptr;

    try {
        size_t out_len = 0;
        uint8_t* result = laser_engine_get_channel_diagnostics(reinterpret_cast<void*>(ptr), &out_len);

        if (!result) return nullptr;

        jbyteArray array = create_byte_array(env, result, out_len);
        gibberlink_free_data(result);
        return array;
    } catch (const std::exception& e) {
        LOGE("Get laser channel diagnostics failed: %s", e.what());
        return nullptr;
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_enableAdaptiveMode(JNIEnv* env, jobject /* this */, jlong ptr) {
    JNIGuard guard(g_laser_mutex);
    if (!ptr) return JNI_FALSE;

    try {
        return laser_engine_enable_adaptive_mode(reinterpret_cast<void*>(ptr)) ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("Enable adaptive mode failed: %s", e.what());
        return JNI_FALSE;
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_disableAdaptiveMode(JNIEnv* env, jobject /* this */, jlong ptr) {
    JNIGuard guard(g_laser_mutex);
    if (!ptr) return JNI_FALSE;

    try {
        return laser_engine_disable_adaptive_mode(reinterpret_cast<void*>(ptr)) ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("Disable adaptive mode failed: %s", e.what());
        return JNI_FALSE;
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_updatePowerProfile(JNIEnv* env, jobject /* this */, jlong ptr, jbyteArray profile) {
    JNIGuard guard(g_laser_mutex);
    if (!ptr || !profile) return JNI_FALSE;

    try {
        auto profile_data = get_byte_array_data(env, profile);
        return laser_engine_update_power_profile(
            reinterpret_cast<void*>(ptr),
            profile_data.data(),
            profile_data.size()
        ) ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("Update power profile failed: %s", e.what());
        return JNI_FALSE;
    }
}

extern "C" JNIEXPORT jbyteArray JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_getCurrentPowerProfile(JNIEnv* env, jobject /* this */, jlong ptr) {
    JNIGuard guard(g_laser_mutex);
    if (!ptr) return nullptr;

    try {
        size_t out_len = 0;
        uint8_t* result = laser_engine_get_current_power_profile(reinterpret_cast<void*>(ptr), &out_len);

        if (!result) return nullptr;

        jbyteArray array = create_byte_array(env, result, out_len);
        gibberlink_free_data(result);
        return array;
    } catch (const std::exception& e) {
        LOGE("Get current power profile failed: %s", e.what());
        return nullptr;
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_emergencyShutdown(JNIEnv* env, jobject /* this */, jlong ptr) {
    JNIGuard guard(g_laser_mutex);
    if (!ptr) return JNI_FALSE;

    try {
        return laser_engine_emergency_shutdown(reinterpret_cast<void*>(ptr)) ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("Emergency shutdown failed: %s", e.what());
        return JNI_FALSE;
    }
}

extern "C" JNIEXPORT jbyteArray JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_getSafetyStats(JNIEnv* env, jobject /* this */, jlong ptr) {
    JNIGuard guard(g_laser_mutex);
    if (!ptr) return nullptr;

    try {
        size_t out_len = 0;
        uint8_t* result = laser_engine_get_safety_stats(reinterpret_cast<void*>(ptr), &out_len);

        if (!result) return nullptr;

        jbyteArray array = create_byte_array(env, result, out_len);
        gibberlink_free_data(result);
        return array;
    } catch (const std::exception& e) {
        LOGE("Get safety stats failed: %s", e.what());
        return nullptr;
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_resetEnergyMonitoring(JNIEnv* env, jobject /* this */, jlong ptr) {
    JNIGuard guard(g_laser_mutex);
    if (!ptr) return JNI_FALSE;

    try {
        return laser_engine_reset_energy_monitoring(reinterpret_cast<void*>(ptr)) ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("Reset energy monitoring failed: %s", e.what());
        return JNI_FALSE;
    }
}

// RangeDetector JNI implementations

extern "C" JNIEXPORT jlong JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_createRangeDetector(JNIEnv* env, jobject /* this */) {
    JNIGuard guard(g_range_detector_mutex);
    try {
        void* ptr = range_detector_create();
        LOGI("Created RangeDetector instance: %p", ptr);
        return reinterpret_cast<jlong>(ptr);
    } catch (const std::exception& e) {
        LOGE("Failed to create RangeDetector: %s", e.what());
        return 0;
    }
}

extern "C" JNIEXPORT void JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_destroyRangeDetector(JNIEnv* env, jobject /* this */, jlong ptr) {
    JNIGuard guard(g_range_detector_mutex);
    if (ptr) {
        range_detector_destroy(reinterpret_cast<void*>(ptr));
        LOGI("Destroyed RangeDetector instance: %p", reinterpret_cast<void*>(ptr));
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_initializeRangeDetector(JNIEnv* env, jobject /* this */, jlong ptr) {
    JNIGuard guard(g_range_detector_mutex);
    if (!ptr) return JNI_FALSE;

    try {
        return range_detector_initialize(reinterpret_cast<void*>(ptr)) ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("RangeDetector initialization failed: %s", e.what());
        return JNI_FALSE;
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_isRangeDetectorActive(JNIEnv* env, jobject /* this */, jlong ptr) {
    JNIGuard guard(g_range_detector_mutex);
    if (!ptr) return JNI_FALSE;

    try {
        return range_detector_is_active(reinterpret_cast<void*>(ptr)) ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("Check RangeDetector active failed: %s", e.what());
        return JNI_FALSE;
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_measureDistance(JNIEnv* env, jobject /* this */, jlong ptr, jfloatArray outDistance, jfloatArray outStrength, jfloatArray outQuality) {
    JNIGuard guard(g_range_detector_mutex);
    if (!ptr || !outDistance || !outStrength || !outQuality) return JNI_FALSE;

    try {
        float distance, strength, quality;
        bool result = range_detector_measure_distance(
            reinterpret_cast<void*>(ptr),
            &distance, &strength, &quality
        );

        if (result) {
            env->SetFloatArrayRegion(outDistance, 0, 1, &distance);
            env->SetFloatArrayRegion(outStrength, 0, 1, &strength);
            env->SetFloatArrayRegion(outQuality, 0, 1, &quality);
        }

        return result ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("Measure distance failed: %s", e.what());
        return JNI_FALSE;
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_measureDistanceAveraged(JNIEnv* env, jobject /* this */, jlong ptr, jint samples, jfloatArray outDistance, jfloatArray outStrength, jfloatArray outQuality) {
    JNIGuard guard(g_range_detector_mutex);
    if (!ptr || !outDistance || !outStrength || !outQuality) return JNI_FALSE;

    try {
        float distance, strength, quality;
        bool result = range_detector_measure_distance_averaged(
            reinterpret_cast<void*>(ptr),
            samples,
            &distance, &strength, &quality
        );

        if (result) {
            env->SetFloatArrayRegion(outDistance, 0, 1, &distance);
            env->SetFloatArrayRegion(outStrength, 0, 1, &strength);
            env->SetFloatArrayRegion(outQuality, 0, 1, &quality);
        }

        return result ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("Measure distance averaged failed: %s", e.what());
        return JNI_FALSE;
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_measureDistanceFast(JNIEnv* env, jobject /* this */, jlong ptr, jfloatArray outDistance, jfloatArray outStrength, jfloatArray outQuality) {
    JNIGuard guard(g_range_detector_mutex);
    if (!ptr || !outDistance || !outStrength || !outQuality) return JNI_FALSE;

    try {
        float distance, strength, quality;
        bool result = range_detector_measure_distance_fast(
            reinterpret_cast<void*>(ptr),
            &distance, &strength, &quality
        );

        if (result) {
            env->SetFloatArrayRegion(outDistance, 0, 1, &distance);
            env->SetFloatArrayRegion(outStrength, 0, 1, &strength);
            env->SetFloatArrayRegion(outQuality, 0, 1, &quality);
        }

        return result ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("Measure distance fast failed: %s", e.what());
        return JNI_FALSE;
    }
}

extern "C" JNIEXPORT void JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_updateRangeDetectorEnvironmentalConditions(JNIEnv* env, jobject /* this */, jlong ptr, jfloat temperature, jfloat humidity, jfloat pressure, jfloat windSpeed, jfloat visibility) {
    JNIGuard guard(g_range_detector_mutex);
    if (!ptr) return;

    try {
        range_detector_update_environmental_conditions(
            reinterpret_cast<void*>(ptr),
            temperature, humidity, pressure, windSpeed, visibility
        );
    } catch (const std::exception& e) {
        LOGE("Update environmental conditions failed: %s", e.what());
    }
}

extern "C" JNIEXPORT void JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_getRangeDetectorEnvironmentalConditions(JNIEnv* env, jobject /* this */, jlong ptr, jfloatArray outTemperature, jfloatArray outHumidity, jfloatArray outPressure, jfloatArray outWindSpeed, jfloatArray outVisibility) {
    JNIGuard guard(g_range_detector_mutex);
    if (!ptr || !outTemperature || !outHumidity || !outPressure || !outWindSpeed || !outVisibility) return;

    try {
        float temperature, humidity, pressure, wind_speed, visibility;
        range_detector_get_environmental_conditions(
            reinterpret_cast<void*>(ptr),
            &temperature, &humidity, &pressure, &wind_speed, &visibility
        );

        env->SetFloatArrayRegion(outTemperature, 0, 1, &temperature);
        env->SetFloatArrayRegion(outHumidity, 0, 1, &humidity);
        env->SetFloatArrayRegion(outPressure, 0, 1, &pressure);
        env->SetFloatArrayRegion(outWindSpeed, 0, 1, &wind_speed);
        env->SetFloatArrayRegion(outVisibility, 0, 1, &visibility);
    } catch (const std::exception& e) {
        LOGE("Get environmental conditions failed: %s", e.what());
    }
}

extern "C" JNIEXPORT jint JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_getCurrentRangeCategory(JNIEnv* env, jobject /* this */, jlong ptr) {
    JNIGuard guard(g_range_detector_mutex);
    if (!ptr) return -1;

    try {
        return range_detector_get_current_range_category(reinterpret_cast<void*>(ptr));
    } catch (const std::exception& e) {
        LOGE("Get current range category failed: %s", e.what());
        return -1;
    }
}

extern "C" JNIEXPORT jint JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_getMeasurementHistorySize(JNIEnv* env, jobject /* this */, jlong ptr) {
    JNIGuard guard(g_range_detector_mutex);
    if (!ptr) return 0;

    try {
        return range_detector_get_measurement_history_size(reinterpret_cast<void*>(ptr));
    } catch (const std::exception& e) {
        LOGE("Get measurement history size failed: %s", e.what());
        return 0;
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_getMeasurementHistory(JNIEnv* env, jobject /* this */, jlong ptr, jint index, jfloatArray outDistance, jfloatArray outStrength, jfloatArray outQuality, jlongArray outTimestamp) {
    JNIGuard guard(g_range_detector_mutex);
    if (!validate_pointer(reinterpret_cast<void*>(ptr), "getMeasurementHistory") ||
        !validate_float_array(env, outDistance, 1, "outDistance", "getMeasurementHistory") ||
        !validate_float_array(env, outStrength, 1, "outStrength", "getMeasurementHistory") ||
        !validate_float_array(env, outQuality, 1, "outQuality", "getMeasurementHistory") ||
        !validate_long_array(env, outTimestamp, 1, "outTimestamp", "getMeasurementHistory")) {
        return JNI_FALSE;
    }

    if (index < 0) {
        LOGE("Invalid history index %d in getMeasurementHistory", index);
        return JNI_FALSE;
    }

    try {
        float distance, strength, quality;
        long timestamp;
        bool result = range_detector_get_measurement_history(
            reinterpret_cast<void*>(ptr),
            index,
            &distance, &strength, &quality, &timestamp
        );

        if (result) {
            env->SetFloatArrayRegion(outDistance, 0, 1, &distance);
            if (env->ExceptionCheck()) {
                LOGE("Exception setting distance array in getMeasurementHistory");
                env->ExceptionClear();
                return JNI_FALSE;
            }
            env->SetFloatArrayRegion(outStrength, 0, 1, &strength);
            if (env->ExceptionCheck()) {
                LOGE("Exception setting strength array in getMeasurementHistory");
                env->ExceptionClear();
                return JNI_FALSE;
            }
            env->SetFloatArrayRegion(outQuality, 0, 1, &quality);
            if (env->ExceptionCheck()) {
                LOGE("Exception setting quality array in getMeasurementHistory");
                env->ExceptionClear();
                return JNI_FALSE;
            }
            env->SetLongArrayRegion(outTimestamp, 0, 1, &timestamp);
            if (env->ExceptionCheck()) {
                LOGE("Exception setting timestamp array in getMeasurementHistory");
                env->ExceptionClear();
                return JNI_FALSE;
            }
        }

        return result ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("Get measurement history failed: %s", e.what());
        return JNI_FALSE;
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_shutdownRangeDetector(JNIEnv* env, jobject /* this */, jlong ptr) {
    JNIGuard guard(g_range_detector_mutex);
    if (!ptr) return JNI_FALSE;

    try {
        return range_detector_shutdown(reinterpret_cast<void*>(ptr)) ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("Shutdown RangeDetector failed: %s", e.what());
        return JNI_FALSE;
    }
}

// Hardware capability detection

extern "C" JNIEXPORT jbyteArray JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_detectHardwareCapabilities(JNIEnv* env, jobject /* this */) {
    JNIGuard guard(g_hardware_mutex);
    try {
        size_t out_len = 0;
        uint8_t* result = detect_hardware_capabilities(&out_len);

        if (!result) return nullptr;

        jbyteArray array = create_byte_array(env, result, out_len);
        gibberlink_free_data(result);
        return array;
    } catch (const std::exception& e) {
        LOGE("Detect hardware capabilities failed: %s", e.what());
        return nullptr;
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_checkUltrasonicHardwareAvailable(JNIEnv* env, jobject /* this */) {
    JNIGuard guard(g_hardware_mutex);
    try {
        return check_ultrasonic_hardware_available() ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("Check ultrasonic hardware failed: %s", e.what());
        return JNI_FALSE;
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_checkLaserHardwareAvailable(JNIEnv* env, jobject /* this */) {
    JNIGuard guard(g_hardware_mutex);
    try {
        return check_laser_hardware_available() ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("Check laser hardware failed: %s", e.what());
        return JNI_FALSE;
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_checkPhotodiodeHardwareAvailable(JNIEnv* env, jobject /* this */) {
    JNIGuard guard(g_hardware_mutex);
    try {
        return check_photodiode_hardware_available() ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("Check photodiode hardware failed: %s", e.what());
        return JNI_FALSE;
    }
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_checkCameraHardwareAvailable(JNIEnv* env, jobject /* this */) {
    JNIGuard guard(g_hardware_mutex);
    try {
        return check_camera_hardware_available() ? JNI_TRUE : JNI_FALSE;
    } catch (const std::exception& e) {
        LOGE("Check camera hardware failed: %s", e.what());
        return JNI_FALSE;
    }
}

// Callback registration (simplified - would need proper JNI callback handling)

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_registerHardwareEventCallback(JNIEnv* env, jobject /* this */, jobject callback) {
    // Store global reference to callback object
    if (g_callback_object != nullptr) {
        env->DeleteGlobalRef(g_callback_object.load());
    }

    if (callback) {
        g_callback_object = env->NewGlobalRef(callback);
    } else {
        g_callback_object = nullptr;
    }

    return JNI_TRUE;
}

extern "C" JNIEXPORT jboolean JNICALL
Java_com_Rgibberlink_RgibberLinkJNI_unregisterHardwareEventCallback(JNIEnv* env, jobject /* this */) {
    if (g_callback_object != nullptr) {
        env->DeleteGlobalRef(g_callback_object.load());
        g_callback_object = nullptr;
    }

    return JNI_TRUE;
}

// JNI_OnLoad for initialization
JNIEXPORT jint JNI_OnLoad(JavaVM* vm, void* /* reserved */) {
    g_java_vm = vm;
    LOGI("GibberLink JNI library loaded");
    return JNI_VERSION_1_6;
}

// JNI_OnUnload for cleanup
JNIEXPORT void JNI_OnUnload(JavaVM* vm, void* /* reserved */) {
    // Clean up global references
    if (g_callback_object != nullptr) {
        JNIEnv* env;
        if (vm->GetEnv(reinterpret_cast<void**>(&env), JNI_VERSION_1_6) == JNI_OK) {
            env->DeleteGlobalRef(g_callback_object.load());
            g_callback_object = nullptr;
        }
    }

    LOGI("GibberLink JNI library unloaded");
}

// Stub implementations for Rust FFI functions (until Rust library is fully implemented)
extern "C" {
    void* gibberlink_create() { return nullptr; }
    void gibberlink_destroy(void* ptr) {}
    bool gibberlink_initiate_handshake(void* ptr) { return false; }
    int gibberlink_get_state(void* ptr) { return 0; }
    const char* gibberlink_receive_nonce(void* ptr, const uint8_t* nonce, size_t nonce_len) { return nullptr; }
    bool gibberlink_process_qr_payload(void* ptr, const uint8_t* qr_data, size_t qr_len) { return false; }
    bool gibberlink_receive_ack(void* ptr) { return false; }
    uint8_t* gibberlink_encrypt_message(void* ptr, const uint8_t* data, size_t data_len, size_t* out_len) { *out_len = 0; return nullptr; }
    uint8_t* gibberlink_decrypt_message(void* ptr, const uint8_t* encrypted_data, size_t encrypted_len, size_t* out_len) { *out_len = 0; return nullptr; }
    bool gibberlink_send_audio_data(void* ptr, const uint8_t* data, size_t data_len) { return false; }
    uint8_t* gibberlink_receive_audio_data(void* ptr, size_t* out_len) { *out_len = 0; return nullptr; }
    bool gibberlink_is_receiving(void* ptr) { return false; }
    const char* gibberlink_generate_qr_code(void* ptr, const uint8_t* payload, size_t payload_len) { return nullptr; }
    uint8_t* gibberlink_decode_qr_code(void* ptr, const uint8_t* qr_data, size_t qr_len, size_t* out_len) { *out_len = 0; return nullptr; }

    void* ultrasonic_beam_engine_create() { return nullptr; }
    void ultrasonic_beam_engine_destroy(void* ptr) {}
    bool ultrasonic_beam_engine_initialize(void* ptr) { return false; }
    uint8_t* ultrasonic_beam_engine_generate_parametric_audio(void* ptr, const uint8_t* data, size_t data_len, size_t* out_len) { *out_len = 0; return nullptr; }
    bool ultrasonic_beam_engine_transmit_sync_pulse(void* ptr, const uint8_t* pattern, size_t pattern_len) { return false; }
    bool ultrasonic_beam_engine_transmit_auth_signal(void* ptr, const uint8_t* challenge, size_t challenge_len, const uint8_t* signature, size_t signature_len) { return false; }
    bool ultrasonic_beam_engine_detect_presence(void* ptr) { return false; }
    bool ultrasonic_beam_engine_transmit_control_data(void* ptr, const uint8_t* data, size_t data_len, uint8_t priority) { return false; }
    uint8_t* ultrasonic_beam_engine_receive_beam_signals(void* ptr, size_t* out_len) { *out_len = 0; return nullptr; }
    uint8_t* ultrasonic_beam_engine_get_config(void* ptr, size_t* out_len) { *out_len = 0; return nullptr; }
    bool ultrasonic_beam_engine_update_config(void* ptr, const uint8_t* config_data, size_t config_len) { return false; }
    uint8_t* ultrasonic_beam_engine_get_channel_diagnostics(void* ptr, size_t* out_len) { *out_len = 0; return nullptr; }
    bool ultrasonic_beam_engine_shutdown(void* ptr) { return false; }

    void* laser_engine_create(const uint8_t* config_data, size_t config_len, const uint8_t* rx_config_data, size_t rx_config_len) { return nullptr; }
    void laser_engine_destroy(void* ptr) {}
    bool laser_engine_initialize(void* ptr) { return false; }
    bool laser_engine_shutdown(void* ptr) { return false; }
    bool laser_engine_transmit_data(void* ptr, const uint8_t* data, size_t data_len) { return false; }
    uint8_t* laser_engine_receive_data(void* ptr, uint64_t timeout_ms, size_t* out_len) { *out_len = 0; return nullptr; }
    bool laser_engine_set_intensity(void* ptr, float intensity) { return false; }
    uint8_t* laser_engine_get_alignment_status(void* ptr, size_t* out_len) { *out_len = 0; return nullptr; }
    bool laser_engine_set_alignment_target(void* ptr, float x, float y) { return false; }
    bool laser_engine_auto_align(void* ptr, uint32_t max_attempts) { return false; }
    uint8_t* laser_engine_get_channel_diagnostics(void* ptr, size_t* out_len) { *out_len = 0; return nullptr; }
    bool laser_engine_enable_adaptive_mode(void* ptr) { return false; }
    bool laser_engine_disable_adaptive_mode(void* ptr) { return false; }
    bool laser_engine_update_power_profile(void* ptr, const uint8_t* profile_data, size_t profile_len) { return false; }
    uint8_t* laser_engine_get_current_power_profile(void* ptr, size_t* out_len) { *out_len = 0; return nullptr; }
    bool laser_engine_emergency_shutdown(void* ptr) { return false; }
    uint8_t* laser_engine_get_safety_stats(void* ptr, size_t* out_len) { *out_len = 0; return nullptr; }
    bool laser_engine_reset_energy_monitoring(void* ptr) { return false; }

    // RangeDetector stub implementations
    void* range_detector_create() { return nullptr; }
    void range_detector_destroy(void* ptr) {}
    bool range_detector_initialize(void* ptr) { return false; }
    bool range_detector_is_active(void* ptr) { return false; }
    bool range_detector_measure_distance(void* ptr, float* out_distance, float* out_strength, float* out_quality) { return false; }
    bool range_detector_measure_distance_averaged(void* ptr, int samples, float* out_distance, float* out_strength, float* out_quality) { return false; }
    bool range_detector_measure_distance_fast(void* ptr, float* out_distance, float* out_strength, float* out_quality) { return false; }
    void range_detector_update_environmental_conditions(void* ptr, float temperature, float humidity, float pressure, float wind_speed, float visibility) {}
    void range_detector_get_environmental_conditions(void* ptr, float* out_temperature, float* out_humidity, float* out_pressure, float* out_wind_speed, float* out_visibility) {}
    int range_detector_get_current_range_category(void* ptr) { return -1; }
    int range_detector_get_measurement_history_size(void* ptr) { return 0; }
    bool range_detector_get_measurement_history(void* ptr, int index, float* out_distance, float* out_strength, float* out_quality, long* out_timestamp) { return false; }
    bool range_detector_shutdown(void* ptr) { return false; }

    uint8_t* detect_hardware_capabilities(size_t* out_len) { *out_len = 0; return nullptr; }
    bool check_ultrasonic_hardware_available() { return false; }
    bool check_laser_hardware_available() { return false; }
    bool check_photodiode_hardware_available() { return false; }
    bool check_camera_hardware_available() { return false; }

    void gibberlink_free_data(uint8_t* data) {}
}