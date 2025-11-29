package com.Rgibberlink

import android.content.Context
import android.content.SharedPreferences
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Log
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import androidx.biometric.BiometricManager
import androidx.biometric.BiometricPrompt
import androidx.core.content.ContextCompat
import java.nio.charset.StandardCharsets
import java.security.KeyStore
import java.security.MessageDigest
import java.util.concurrent.Executor
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.IvParameterSpec
import kotlin.math.min

/**
 * PermissionManager handles all permission-related logic including PIN validation,
 * biometric authentication, security policies, and lockout management.
 */
class PermissionManager(private val context: Context) {

    companion object {
        private const val TAG = "PermissionManager"
        private const val PREF_PIN = "auth_pin"
        private const val PREF_PIN_CHANGED = "pin_changed"
        private const val PREF_FAILED_ATTEMPTS = "failed_attempts"
        private const val PREF_LOCKOUT_TIME = "lockout_time"
        private const val PREF_DEFAULT_PIN = "9999"
        const val MAX_FAILED_ATTEMPTS = 3
        private const val LOCKOUT_DURATION_MS = 300000L // 5 minutes

        // Permission constants matching the specs
        const val PERMISSION_DISCUSSIONS = "discussions"
        const val PERMISSION_ACCESS_AUTH = "access_auth"
        const val PERMISSION_COMMANDS = "commands"
        const val PERMISSION_COUPLINGS = "couplings"
        const val PERMISSION_OTHERS = "others"

        // Duration constants
        const val DURATION_30S = 30
        const val DURATION_5MIN = 300
        const val DURATION_30MIN = 1800
        const val DURATION_SESSION = -1 // Post-usage expiration

        // Scope constants
        const val SCOPE_THIS_NODE = "this_node"
        const val SCOPE_THIS_GROUP = "this_group"
        const val SCOPE_ALL_VISIBLE = "all_visible"
    }

    private val prefs: SharedPreferences = context.getSharedPreferences("gibberlink_auth", Context.MODE_PRIVATE)

    // Biometric components
    private var biometricPrompt: BiometricPrompt? = null
    private var promptInfo: BiometricPrompt.PromptInfo? = null
    private var biometricAvailable = false
    private var biometricCallback: ((Boolean) -> Unit)? = null

    init {
        initBiometric()
    }

    private fun initBiometric() {
        val biometricManager = BiometricManager.from(context)
        biometricAvailable = when (biometricManager.canAuthenticate(BiometricManager.Authenticators.BIOMETRIC_STRONG)) {
            BiometricManager.BIOMETRIC_SUCCESS -> true
            else -> false
        }

        if (biometricAvailable) {
            val executor: Executor = ContextCompat.getMainExecutor(context)
            biometricPrompt = BiometricPrompt(context as AppCompatActivity, executor,
                object : BiometricPrompt.AuthenticationCallback() {
                    override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                        super.onAuthenticationSucceeded(result)
                        biometricCallback?.invoke(true)
                    }

                    override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
                        super.onAuthenticationError(errorCode, errString)
                        biometricCallback?.invoke(false)
                    }
                })

            promptInfo = BiometricPrompt.PromptInfo.Builder()
                .setTitle("Confirm Sensitive Permissions")
                .setSubtitle("Use biometric authentication for enhanced security")
                .setNegativeButtonText("Use PIN")
                .build()
        }
    }

    fun isBiometricAvailable(): Boolean = biometricAvailable

    fun authenticateWithBiometric(callback: (Boolean) -> Unit) {
        biometricCallback = callback
        biometricPrompt?.authenticate(promptInfo!!)
    }

    fun validatePin(pin: String): Boolean {
        if (pin.length != 4 || !pin.all { it.isDigit() }) {
            return false
        }

        val storedPin = prefs.getString(PREF_PIN, PREF_DEFAULT_PIN) ?: PREF_DEFAULT_PIN
        return storedPin == pin
    }

    fun handleFailedPinAttempt(): Boolean {
        val failedAttempts = prefs.getInt(PREF_FAILED_ATTEMPTS, 0) + 1
        prefs.edit().putInt(PREF_FAILED_ATTEMPTS, failedAttempts).apply()

        if (failedAttempts >= MAX_FAILED_ATTEMPTS) {
            prefs.edit().putLong(PREF_LOCKOUT_TIME, System.currentTimeMillis()).apply()
            return true // Device locked
        }
        return false
    }

    fun isDeviceLocked(): Boolean {
        val failedAttempts = prefs.getInt(PREF_FAILED_ATTEMPTS, 0)
        val lockoutTime = prefs.getLong(PREF_LOCKOUT_TIME, 0)

        if (failedAttempts >= MAX_FAILED_ATTEMPTS) {
            val timeSinceLockout = System.currentTimeMillis() - lockoutTime
            return timeSinceLockout < LOCKOUT_DURATION_MS
        }
        return false
    }

    fun getRemainingAttempts(): Int {
        val failedAttempts = prefs.getInt(PREF_FAILED_ATTEMPTS, 0)
        return min(MAX_FAILED_ATTEMPTS - failedAttempts, MAX_FAILED_ATTEMPTS)
    }

    fun resetFailedAttempts() {
        prefs.edit()
            .putInt(PREF_FAILED_ATTEMPTS, 0)
            .putLong(PREF_LOCKOUT_TIME, 0)
            .apply()
    }

    fun shouldUseBiometric(permissions: List<String>): Boolean {
        return biometricAvailable && (permissions.contains(PERMISSION_COMMANDS) ||
                                    permissions.contains(PERMISSION_ACCESS_AUTH))
    }

    fun validatePermissions(permissions: List<String>): List<String> {
        // Apply security policies (e.g., ensure discussions is always included)
        val validated = permissions.toMutableList()
        if (!validated.contains(PERMISSION_DISCUSSIONS)) {
            validated.add(PERMISSION_DISCUSSIONS)
        }
        return validated.distinct()
    }

    fun checkPinChangeRequirement(pin: String) {
        val pinChanged = prefs.getBoolean(PREF_PIN_CHANGED, false)

        if (!pinChanged) {
            if (pin != PREF_DEFAULT_PIN) {
                // User changed PIN, mark as changed
                prefs.edit()
                    .putString(PREF_PIN, pin)
                    .putBoolean(PREF_PIN_CHANGED, true)
                    .apply()
            }
        }
    }

    fun isFaceValidationRequired(permissions: List<String>): Boolean {
        return permissions.contains(PERMISSION_COMMANDS) ||
               permissions.contains(PERMISSION_ACCESS_AUTH) ||
               permissions.contains(PERMISSION_COUPLINGS)
    }

    fun isFaceValidationEnabled(): Boolean {
        val appPrefs = context.getSharedPreferences("gibberlink_app", Context.MODE_PRIVATE)
        return appPrefs.getBoolean("face_validation_enabled", false)
    }

    fun createAuthorizationData(
        peerIdentity: String,
        permissions: List<String>,
        duration: Int,
        scope: String,
        biometricUsed: Boolean
    ): AuthorizationData {
        return AuthorizationData(
            peerIdentity = peerIdentity,
            permissions = permissions,
            durationSeconds = duration,
            scope = scope,
            grantedAt = System.currentTimeMillis(),
            biometricUsed = biometricUsed
        )
    }

    fun logAuthenticationEvent(authData: AuthorizationData, granted: Boolean) {
        // In a full implementation, this would integrate with the signed logging system
        Log.i(TAG, "Authorization ${if (granted) "granted" else "denied"} for peer ${authData.peerIdentity} with permissions: ${authData.permissions}")
    }
}