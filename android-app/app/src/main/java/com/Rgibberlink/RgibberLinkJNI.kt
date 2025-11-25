package com.Rgibberlink

class RgibberLinkJNI {
    companion object {
        init {
            System.loadLibrary("gibberlink-jni")
        }

        // Protocol operations
        external fun createGibberLink(): Long
        external fun destroyGibberLink(ptr: Long)
        external fun initiateHandshake(ptr: Long): Boolean
        external fun getState(ptr: Long): Int
        external fun receiveNonce(ptr: Long, nonce: ByteArray): String?
        external fun processQrPayload(ptr: Long, qrData: ByteArray): Boolean
        external fun receiveAck(ptr: Long): Boolean

        // Crypto operations
        external fun encryptMessage(ptr: Long, data: ByteArray): ByteArray?
        external fun decryptMessage(ptr: Long, encryptedData: ByteArray): ByteArray?

        // Audio operations
        external fun sendAudioData(ptr: Long, data: ByteArray): Boolean
        external fun receiveAudioData(ptr: Long): ByteArray?
        external fun isReceiving(ptr: Long): Boolean

        // Visual operations
        external fun generateQrCode(ptr: Long, payload: ByteArray): String?
        external fun decodeQrCode(ptr: Long, qrData: ByteArray): ByteArray?

        // UltrasonicBeamEngine operations
        external fun createUltrasonicBeamEngine(): Long
        external fun destroyUltrasonicBeamEngine(ptr: Long)
        external fun initializeUltrasonicBeamEngine(ptr: Long): Boolean
        external fun generateParametricAudio(ptr: Long, data: ByteArray): ByteArray?
        external fun transmitSyncPulse(ptr: Long, pattern: ByteArray): Boolean
        external fun transmitAuthSignal(ptr: Long, challenge: ByteArray, signature: ByteArray): Boolean
        external fun detectPresence(ptr: Long): Boolean
        external fun transmitControlData(ptr: Long, data: ByteArray, priority: Int): Boolean
        external fun receiveBeamSignals(ptr: Long): ByteArray?
        external fun getUltrasonicBeamConfig(ptr: Long): ByteArray?
        external fun updateUltrasonicBeamConfig(ptr: Long, config: ByteArray): Boolean
        external fun getUltrasonicChannelDiagnostics(ptr: Long): ByteArray?
        external fun shutdownUltrasonicBeamEngine(ptr: Long): Boolean

        // LaserEngine operations
        external fun createLaserEngine(config: ByteArray, rxConfig: ByteArray): Long
        external fun destroyLaserEngine(ptr: Long)
        external fun initializeLaserEngine(ptr: Long): Boolean
        external fun shutdownLaserEngine(ptr: Long): Boolean
        external fun transmitLaserData(ptr: Long, data: ByteArray): Boolean
        external fun receiveLaserData(ptr: Long, timeoutMs: Int): ByteArray?
        external fun setLaserIntensity(ptr: Long, intensity: Float): Boolean
        external fun getAlignmentStatus(ptr: Long): ByteArray?
        external fun setAlignmentTarget(ptr: Long, x: Float, y: Float): Boolean
        external fun autoAlign(ptr: Long, maxAttempts: Int): Boolean
        external fun getLaserChannelDiagnostics(ptr: Long): ByteArray?
        external fun enableAdaptiveMode(ptr: Long): Boolean
        external fun disableAdaptiveMode(ptr: Long): Boolean
        external fun updatePowerProfile(ptr: Long, profile: ByteArray): Boolean
        external fun getCurrentPowerProfile(ptr: Long): ByteArray?
        external fun emergencyShutdown(ptr: Long): Boolean
        external fun getSafetyStats(ptr: Long): ByteArray?
        external fun resetEnergyMonitoring(ptr: Long): Boolean

        // RangeDetector operations
        external fun createRangeDetector(): Long
        external fun destroyRangeDetector(ptr: Long)
        external fun initializeRangeDetector(ptr: Long): Boolean
        external fun isRangeDetectorActive(ptr: Long): Boolean
        external fun measureDistance(ptr: Long, outDistance: FloatArray, outStrength: FloatArray, outQuality: FloatArray): Boolean
        external fun measureDistanceAveraged(ptr: Long, samples: Int, outDistance: FloatArray, outStrength: FloatArray, outQuality: FloatArray): Boolean
        external fun measureDistanceFast(ptr: Long, outDistance: FloatArray, outStrength: FloatArray, outQuality: FloatArray): Boolean
        external fun updateRangeDetectorEnvironmentalConditions(ptr: Long, temperature: Float, humidity: Float, pressure: Float, windSpeed: Float, visibility: Float)
        external fun getRangeDetectorEnvironmentalConditions(ptr: Long, outTemperature: FloatArray, outHumidity: FloatArray, outPressure: FloatArray, outWindSpeed: FloatArray, outVisibility: FloatArray)
        external fun getCurrentRangeCategory(ptr: Long): Int
        external fun getMeasurementHistorySize(ptr: Long): Int
        external fun getMeasurementHistory(ptr: Long, index: Int, outDistance: FloatArray, outStrength: FloatArray, outQuality: FloatArray, outTimestamp: LongArray): Boolean
        external fun shutdownRangeDetector(ptr: Long): Boolean

        // Hardware capability detection
        external fun detectHardwareCapabilities(): ByteArray?
        external fun checkUltrasonicHardwareAvailable(): Boolean
        external fun checkLaserHardwareAvailable(): Boolean
        external fun checkPhotodiodeHardwareAvailable(): Boolean
        external fun checkCameraHardwareAvailable(): Boolean

        // Real-time callbacks (callback registration)
        external fun registerHardwareEventCallback(callback: Any): Boolean
        external fun unregisterHardwareEventCallback(): Boolean

        // Constants
        const val STATE_IDLE = 0
        const val STATE_SENDING_NONCE = 1
        const val STATE_WAITING_FOR_QR = 2
        const val STATE_SENDING_ACK = 3
        const val STATE_CONNECTED = 4
        const val STATE_ERROR = 5

        // Hardware error codes
        const val ERROR_NONE = 0
        const val ERROR_HARDWARE_UNAVAILABLE = 1
        const val ERROR_INVALID_PARAMETERS = 2
        const val ERROR_TRANSMISSION_FAILED = 3
        const val ERROR_RECEPTION_FAILED = 4
        const val ERROR_SAFETY_VIOLATION = 5
        const val ERROR_ALIGNMENT_LOST = 6
        const val ERROR_TIMEOUT = 7
        const val ERROR_THREAD_SAFETY = 8
    }
}