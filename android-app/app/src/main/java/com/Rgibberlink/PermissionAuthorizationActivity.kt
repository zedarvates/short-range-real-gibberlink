package com.Rgibberlink

import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Log
import android.view.View
import android.widget.*
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

class PermissionAuthorizationActivity : AppCompatActivity() {

    companion object {
        private const val TAG = "PermissionAuth"
        private const val PREF_PIN = "auth_pin"
        private const val PREF_PIN_CHANGED = "pin_changed"
        private const val PREF_FAILED_ATTEMPTS = "failed_attempts"
        private const val PREF_LOCKOUT_TIME = "lockout_time"
        private const val PREF_DEFAULT_PIN = "9999"
        private const val MAX_FAILED_ATTEMPTS = 3
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

    // UI Components
    private lateinit var peerFingerprint: TextView
    private lateinit var irStatus: TextView
    private lateinit var ultrasoundStatus: TextView
    private lateinit var distanceText: TextView
    private lateinit var angleText: TextView
    private lateinit var warningCard: View
    private lateinit var warningText: TextView
    private lateinit var pinInput: EditText
    private lateinit var biometricNotice: TextView
    private lateinit var lockoutWarning: TextView
    private lateinit var btnAuthorize: Button
    private lateinit var btnDeny: Button

    // Permission checkboxes
    private lateinit var permissionDiscussions: CheckBox
    private lateinit var permissionAccessAuth: CheckBox
    private lateinit var permissionCommands: CheckBox
    private lateinit var permissionCouplings: CheckBox
    private lateinit var permissionOthers: CheckBox

    // Radio groups
    private lateinit var durationGroup: RadioGroup
    private lateinit var scopeGroup: RadioGroup

    // Preferences
    private lateinit var prefs: android.content.SharedPreferences

    // Biometric
    private lateinit var biometricPrompt: BiometricPrompt
    private lateinit var promptInfo: BiometricPrompt.PromptInfo
    private var biometricAvailable = false
    private var requiresBiometric = false

    // Connection data from intent
    private var peerIdentity = "GL-AB12-CDEF"
    private var estimatedDistance = 75.0f
    private var estimatedAngle = 12.0f
    private var irOk = true
    private var ultrasoundOk = true

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_permission_authorization)

        prefs = getSharedPreferences("gibberlink_auth", Context.MODE_PRIVATE)

        // Get connection data from intent
        peerIdentity = intent.getStringExtra("peer_identity") ?: peerIdentity
        estimatedDistance = intent.getFloatExtra("distance", estimatedDistance)
        estimatedAngle = intent.getFloatExtra("angle", estimatedAngle)
        irOk = intent.getBooleanExtra("ir_ok", irOk)
        ultrasoundOk = intent.getBooleanExtra("ultrasound_ok", ultrasoundOk)

        initBiometric()
        initializeViews()
        setupSecurityPolicies()
        checkLockoutStatus()
        updatePinHint()
    }

    private fun initBiometric() {
        val biometricManager = BiometricManager.from(this)
        biometricAvailable = when (biometricManager.canAuthenticate(BiometricManager.Authenticators.BIOMETRIC_STRONG)) {
            BiometricManager.BIOMETRIC_SUCCESS -> true
            else -> false
        }

        if (biometricAvailable) {
            val executor: Executor = ContextCompat.getMainExecutor(this)
            biometricPrompt = BiometricPrompt(this, executor,
                object : BiometricPrompt.AuthenticationCallback() {
                    override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                        super.onAuthenticationSucceeded(result)
                        onBiometricSuccess()
                    }

                    override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
                        super.onAuthenticationError(errorCode, errString)
                        Toast.makeText(this@PermissionAuthorizationActivity,
                            "Biometric authentication failed", Toast.LENGTH_SHORT).show()
                    }
                })

            promptInfo = BiometricPrompt.PromptInfo.Builder()
                .setTitle("Confirm Sensitive Permissions")
                .setSubtitle("Use biometric authentication for enhanced security")
                .setNegativeButtonText("Use PIN")
                .build()
        }
    }

    private fun initializeViews() {
        peerFingerprint = findViewById(R.id.peerFingerprint)
        irStatus = findViewById(R.id.irStatus)
        ultrasoundStatus = findViewById(R.id.ultrasoundStatus)
        distanceText = findViewById(R.id.distanceText)
        angleText = findViewById(R.id.angleText)
        warningCard = findViewById(R.id.warningCard)
        warningText = findViewById(R.id.warningText)
        pinInput = findViewById(R.id.pinInput)
        biometricNotice = findViewById(R.id.biometricNotice)
        lockoutWarning = findViewById(R.id.lockoutWarning)
        btnAuthorize = findViewById(R.id.btnAuthorize)
        btnDeny = findViewById(R.id.btnDeny)

        permissionDiscussions = findViewById(R.id.permissionDiscussions)
        permissionAccessAuth = findViewById(R.id.permissionAccessAuth)
        permissionCommands = findViewById(R.id.permissionCommands)
        permissionCouplings = findViewById(R.id.permissionCouplings)
        permissionOthers = findViewById(R.id.permissionOthers)

        durationGroup = findViewById(R.id.durationGroup)
        scopeGroup = findViewById(R.id.scopeGroup)

        // Set initial data
        peerFingerprint.text = peerIdentity
        updateConnectionStatus()
        updateDistanceAngle()

        // Set click listeners
        btnAuthorize.setOnClickListener { handleAuthorization() }
        btnDeny.setOnClickListener { denyAuthorization() }

        // Permission change listeners for policy enforcement
        permissionAccessAuth.setOnCheckedChangeListener { _, isChecked -> onPermissionChanged() }
        permissionCommands.setOnCheckedChangeListener { _, isChecked -> onPermissionChanged() }
        permissionCouplings.setOnCheckedChangeListener { _, isChecked -> onPermissionChanged() }
    }

    private fun updateConnectionStatus() {
        irStatus.text = if (irOk) "✅ OK" else "❌ FAIL"
        irStatus.setTextColor(resources.getColor(
            if (irOk) android.R.color.holo_green_dark else android.R.color.holo_red_dark))

        ultrasoundStatus.text = if (ultrasoundOk) "✅ OK" else "❌ FAIL"
        ultrasoundStatus.setTextColor(resources.getColor(
            if (ultrasoundOk) android.R.color.holo_green_dark else android.R.color.holo_red_dark))
    }

    private fun updateDistanceAngle() {
        distanceText.text = "%.1fm".format(estimatedDistance)
        angleText.text = "%.1f°".format(estimatedAngle)
    }

    private fun setupSecurityPolicies() {
        // Default policy: discussions only
        applyMinimalPolicy()

        // Check for PIN strength warnings
        updateSecurityWarnings()
    }

    private fun applyMinimalPolicy() {
        // By default, only enable discussions
        permissionDiscussions.isChecked = true
        updateBiometricRequirement()
    }

    private fun onPermissionChanged() {
        updateBiometricRequirement()
        updateSecurityWarnings()
    }

    private fun updateBiometricRequirement() {
        requiresBiometric = permissionCommands.isChecked || permissionAccessAuth.isChecked
        biometricNotice.visibility = if (biometricAvailable && requiresBiometric) View.VISIBLE else View.GONE
    }

    private fun updateSecurityWarnings() {
        val warnings = mutableListOf<String>()

        // Check PIN strength
        val currentPin = pinInput.text.toString()
        if (currentPin.length < 4) {
            warnings.add("PIN must be 4 digits")
        } else if (currentPin == PREF_DEFAULT_PIN) {
            warnings.add("Default PIN is weak - change immediately")
        }

        // Check critical permissions
        if (permissionCommands.isChecked || permissionAccessAuth.isChecked) {
            warnings.add("Critical permissions enabled - requires strong authentication")
        }

        if (warnings.isEmpty()) {
            warningCard.visibility = View.GONE
        } else {
            warningText.text = warnings.joinToString("\n• ", "• ")
            warningCard.visibility = View.VISIBLE
        }
    }

    private fun checkLockoutStatus() {
        val failedAttempts = prefs.getInt(PREF_FAILED_ATTEMPTS, 0)
        val lockoutTime = prefs.getLong(PREF_LOCKOUT_TIME, 0)

        if (failedAttempts >= MAX_FAILED_ATTEMPTS) {
            val timeSinceLockout = System.currentTimeMillis() - lockoutTime
            if (timeSinceLockout < LOCKOUT_DURATION_MS) {
                lockDevice()
                return
            } else {
                // Lockout period passed, reset
                prefs.edit()
                    .putInt(PREF_FAILED_ATTEMPTS, 0)
                    .putLong(PREF_LOCKOUT_TIME, 0)
                    .apply()
            }
        }

        if (failedAttempts > 0) {
            lockoutWarning.text = "🔐 Incorrect PIN attempts: $failedAttempts/$MAX_FAILED_ATTEMPTS"
            lockoutWarning.visibility = if (failedAttempts >= MAX_FAILED_ATTEMPTS - 1) View.VISIBLE else View.GONE
        }
    }

    private fun lockDevice() {
        pinInput.isEnabled = false
        btnAuthorize.isEnabled = false
        lockoutWarning.text = "🔐 Device locked due to too many failed attempts\nTry again in 5 minutes"
        lockoutWarning.visibility = View.VISIBLE
        btnAuthorize.text = "LOCKED"

        Toast.makeText(this, "Device locked due to failed PIN attempts", Toast.LENGTH_LONG).show()
    }

    private fun updatePinHint() {
        val pinChanged = prefs.getBoolean(PREF_PIN_CHANGED, false)
        if (!pinChanged) {
            Toast.makeText(this, "Default PIN is 9999 - please change it after first use", Toast.LENGTH_LONG).show()
        }
    }

    private fun handleAuthorization() {
        val enteredPin = pinInput.text.toString()

        if (!validatePin(enteredPin)) {
            handleFailedPinAttempt()
            return
        }

        // Check biometric requirement for sensitive permissions
        if (requiresBiometric && biometricAvailable) {
            biometricPrompt.authenticate(promptInfo)
            return // Wait for biometric callback
        }

        // PIN validation successful, proceed with authorization
        completeAuthorization()
    }

    private fun validatePin(pin: String): Boolean {
        if (pin.length != 4 || !pin.all { it.isDigit() }) {
            Toast.makeText(this, "PIN must be 4 digits", Toast.LENGTH_SHORT).show()
            return false
        }

        val storedPin = prefs.getString(PREF_PIN, PREF_DEFAULT_PIN) ?: PREF_DEFAULT_PIN
        return storedPin == pin
    }

    private fun handleFailedPinAttempt() {
        val failedAttempts = prefs.getInt(PREF_FAILED_ATTEMPTS, 0) + 1

        prefs.edit()
            .putInt(PREF_FAILED_ATTEMPTS, failedAttempts)
            .apply()

        if (failedAttempts >= MAX_FAILED_ATTEMPTS) {
            prefs.edit().putLong(PREF_LOCKOUT_TIME, System.currentTimeMillis()).apply()
            lockDevice()
        } else {
            val remaining = MAX_FAILED_ATTEMPTS - failedAttempts
            Toast.makeText(this, "Incorrect PIN. $remaining attempts remaining", Toast.LENGTH_SHORT).show()
            updateLockoutWarning(failedAttempts)
        }
    }

    private fun updateLockoutWarning(attempts: Int) {
        lockoutWarning.text = "🔐 Incorrect PIN attempts: $attempts/$MAX_FAILED_ATTEMPTS"
        lockoutWarning.visibility = if (attempts >= MAX_FAILED_ATTEMPTS - 1) View.VISIBLE else View.GONE
    }

    private fun onBiometricSuccess() {
        completeAuthorization()
    }

    private fun completeAuthorization() {
        // Reset failed attempts on success
        prefs.edit()
            .putInt(PREF_FAILED_ATTEMPTS, 0)
            .putLong(PREF_LOCKOUT_TIME, 0)
            .apply()

        // Check if face validation is required for sensitive permissions
        if (requiresSensitivePermissions() && isFaceValidationEnabled()) {
            // Request face validation before completing authorization
            requestFaceValidation { success ->
                if (success) {
                    completeAuthorization()
                } else {
                    // Show error and allow retry or cancel
                    showFaceValidationFailed()
                }
            }
            return // Wait for face validation completion
        }

        // No face validation required, proceed normally
        completeAuthorization()
    }

    private fun denyAuthorization() {
        val intent = Intent().apply {
            putExtra("authorized", false)
            putExtra("reason", "user_denied")
        }
        setResult(RESULT_CANCELED, intent)
        finish()
    }

    private fun collectAuthorizationData(): AuthorizationData {
        val duration = when (durationGroup.checkedRadioButtonId) {
            R.id.duration30s -> DURATION_30S
            R.id.duration5min -> DURATION_5MIN
            R.id.duration30min -> DURATION_30MIN
            R.id.durationSession -> DURATION_SESSION
            else -> DURATION_5MIN
        }

        val scope = when (scopeGroup.checkedRadioButtonId) {
            R.id.scopeThisNode -> SCOPE_THIS_NODE
            R.id.scopeThisGroup -> SCOPE_THIS_GROUP
            R.id.scopeAllVisible -> SCOPE_ALL_VISIBLE
            else -> SCOPE_THIS_NODE
        }

        val permissions = mutableListOf<String>()
        if (permissionDiscussions.isChecked) permissions.add(PERMISSION_DISCUSSIONS)
        if (permissionAccessAuth.isChecked) permissions.add(PERMISSION_ACCESS_AUTH)
        if (permissionCommands.isChecked) permissions.add(PERMISSION_COMMANDS)
        if (permissionCouplings.isChecked) permissions.add(PERMISSION_COUPLINGS)
        if (permissionOthers.isChecked) permissions.add(PERMISSION_OTHERS)

        return AuthorizationData(
            peerIdentity = peerIdentity,
            permissions = permissions,
            durationSeconds = duration,
            scope = scope,
            grantedAt = System.currentTimeMillis(),
            biometricUsed = requiresBiometric && biometricAvailable
        )
    }

    private fun checkPinChangeRequirement() {
        val enteredPin = pinInput.text.toString()
        val pinChanged = prefs.getBoolean(PREF_PIN_CHANGED, false)

        if (!pinChanged) {
            if (enteredPin != PREF_DEFAULT_PIN) {
                // User changed PIN, mark as changed
                prefs.edit()
                    .putString(PREF_PIN, enteredPin)
                    .putBoolean(PREF_PIN_CHANGED, true)
                    .apply()
                Toast.makeText(this, "PIN changed successfully", Toast.LENGTH_SHORT).show()
            } else {
                // Still using default, prompt to change
                Toast.makeText(this, "Please change your PIN from the default", Toast.LENGTH_LONG).show()
            }
        }
    }

    private fun requiresSensitivePermissions(): Boolean {
        return permissionCommands.isChecked || permissionAccessAuth.isChecked || permissionCouplings.isChecked
    }

    private fun isFaceValidationEnabled(): Boolean {
        val appPrefs = getSharedPreferences("gibberlink_app", Context.MODE_PRIVATE)
        return appPrefs.getBoolean("face_validation_enabled", false)
    }

    private fun requestFaceValidation(callback: (Boolean) -> Unit) {
        val intent = Intent(this, FaceValidationActivity::class.java)
        faceValidationLauncher.launch(intent)

        // Store callback for later use
        faceValidationCallback = callback
    }

    private fun showFaceValidationFailed() {
        val builder = android.app.AlertDialog.Builder(this)
        builder.setTitle("Face Validation Required")
        builder.setMessage("Face validation is required for sensitive permissions. Please try again or contact administrator.")
        builder.setPositiveButton("Retry") { _, _ ->
            // Retry face validation
            requestFaceValidation { success ->
                if (success) {
                    completeAuthorizationFinal()
                } else {
                    showFaceValidationFailed()
                }
            }
        }
        builder.setNegativeButton("Cancel") { _, _ ->
            denyAuthorization()
        }
        builder.show()
    }

    // Activity result handler for face validation
    private val faceValidationLauncher = registerForActivityResult(
        ActivityResultContracts.StartActivityForResult()
    ) { result ->
        val success = result.resultCode == RESULT_OK
        faceValidationCallback?.invoke(success)
        faceValidationCallback = null
    }

    private var faceValidationCallback: ((Boolean) -> Unit)? = null

    private fun completeAuthorizationFinal() {
        // Check if this is first PIN change
        checkPinChangeRequirement()

        // Collect authorization data
        val authData = collectAuthorizationData()

        // Return result to caller
        val intent = Intent().apply {
            putExtra("authorized", true)
            putExtra("authorization_data", authData.toByteArray())
        }
        setResult(RESULT_OK, intent)

        // Log authentication for audit
        logAuthenticationEvent(authData, true)

        finish()
    }

    private fun logAuthenticationEvent(authData: AuthorizationData, granted: Boolean) {
        // In a full implementation, this would integrate with the signed logging system
        Log.i(TAG, "Authorization ${if (granted) "granted" else "denied"} for peer $peerIdentity with permissions: ${authData.permissions}")
    }
}

// Data class for authorization results
data class AuthorizationData(
    val peerIdentity: String,
    val permissions: List<String>,
    val durationSeconds: Int,
    val scope: String,
    val grantedAt: Long,
    val biometricUsed: Boolean
) {
    fun toByteArray(): ByteArray {
        // Serialize to JSON for simplicity (in production, use proper serialization)
        return "{}".toByteArray(StandardCharsets.UTF_8) // Placeholder
    }
}
