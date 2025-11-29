package com.Rgibberlink

import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.util.Log
import android.view.View
import android.widget.*
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import androidx.biometric.BiometricManager
import androidx.biometric.BiometricPrompt
import androidx.core.content.ContextCompat
import java.util.concurrent.Executor

class MainActivity : AppCompatActivity() {

    companion object {
        private const val TAG = "MainActivity"
        private const val PREF_APP_PIN = "app_pin"
        private const val PREF_BIOMETRIC_ENABLED = "biometric_enabled"
        private const val PREF_FIRST_RUN = "first_run"
        private const val DEFAULT_PIN = "0000"
        private const val MAX_FAILED_ATTEMPTS = 5
        private const val LOCKOUT_DURATION_MS = 300000L // 5 minutes

        // Shared preference keys
        private const val PREF_FAILED_ATTEMPTS = "app_failed_attempts"
        private const val PREF_LOCKOUT_TIME = "app_lockout_time"
    }

    // UI Components
    private lateinit var appIcon: ImageView
    private lateinit var appTitle: TextView
    private lateinit var authMessage: TextView
    private lateinit var pinInput: EditText
    private lateinit var btnAuthenticate: Button
    private lateinit var btnSetup: Button
    private lateinit var biometricContainer: View
    private lateinit var biometricToggle: Switch
    private lateinit var pinCard: View
    private lateinit var lockoutWarning: TextView
    private lateinit var progressBar: ProgressBar

    // Encrypted Preferences
    private lateinit var prefs: android.content.SharedPreferences

    // Biometric
    private lateinit var biometricPrompt: BiometricPrompt
    private lateinit var promptInfo: BiometricPrompt.PromptInfo
    private var biometricAvailable = false
    private var isFirstRun = true

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        // Initialize encrypted preferences
        val masterKey = MasterKey.Builder(applicationContext)
            .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
            .build()

        prefs = EncryptedSharedPreferences.create(
            applicationContext,
            "gibberlink_secure_prefs",
            masterKey,
            EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
            EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
        )

        // Check if this is first run
        isFirstRun = prefs.getBoolean(PREF_FIRST_RUN, true)

        // Initialize UI components
        initializeViews()

        // Set up biometric authentication
        setupBiometric()

        // Check authentication state
        checkAuthenticationState()
    }

    private fun initializeViews() {
        appIcon = findViewById(R.id.appIcon)
        appTitle = findViewById(R.id.appTitle)
        authMessage = findViewById(R.id.authMessage)
        pinInput = findViewById(R.id.pinInput)
        btnAuthenticate = findViewById(R.id.btnAuthenticate)
        btnSetup = findViewById(R.id.btnSetup)
        biometricContainer = findViewById(R.id.biometricContainer)
        biometricToggle = findViewById(R.id.biometricToggle)
        pinCard = findViewById(R.id.pinCard)
        lockoutWarning = findViewById(R.id.lockoutWarning)
        progressBar = findViewById(R.id.progressBar)

        // Set click listeners
        btnAuthenticate.setOnClickListener { onAuthenticateButtonClick() }
        btnSetup.setOnClickListener { showSetupDialog() }

        // Set initial UI state
        updateAuthUI()
    }

    private fun setupBiometric() {
        val biometricManager = BiometricManager.from(this)
        biometricAvailable = when (biometricManager.canAuthenticate(BiometricManager.Authenticators.BIOMETRIC_STRONG)) {
            BiometricManager.BIOMETRIC_SUCCESS -> true
            BiometricManager.BIOMETRIC_ERROR_NONE_ENROLLED -> {
                // Show setup message
                authMessage.text = "Biometric authentication available but not enrolled. Please set up fingerprint/face recognition in device settings."
                false
            }
            else -> {
                authMessage.text = "Biometric authentication not available on this device."
                false
            }
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
                        onBiometricError(errorCode, errString.toString())
                    }

                    override fun onAuthenticationFailed() {
                        super.onAuthenticationFailed()
                        onBiometricFailed()
                    }
                })

            promptInfo = BiometricPrompt.PromptInfo.Builder()
                .setTitle("GibberLink Security")
                .setSubtitle("Use biometric authentication to access the app")
                .setNegativeButtonText("Use PIN")
                .build()
        }
    }

    private fun checkAuthenticationState() {
        // Check if device is locked due to failed attempts
        if (isDeviceLocked()) {
            showLockedUI()
            return
        }

        // If biometric is enabled and available, automatically prompt
        val biometricEnabled = prefs.getBoolean(PREF_BIOMETRIC_ENABLED, false) && biometricAvailable

        if (biometricEnabled && !isFirstRun) {
            // Auto-prompt biometric authentication
            biometricPrompt.authenticate(promptInfo)
            authMessage.text = "Use fingerprint/face recognition or tap 'Use PIN' to authenticate"
            btnAuthenticate.visibility = View.GONE
            btnSetup.visibility = View.GONE
        } else {
            // Show PIN authentication UI
            showPinAuthentication()
        }
    }

    private fun isDeviceLocked(): Boolean {
        val failedAttempts = prefs.getInt(PREF_FAILED_ATTEMPTS, 0)
        val lockoutTime = prefs.getLong(PREF_LOCKOUT_TIME, 0)

        if (failedAttempts >= MAX_FAILED_ATTEMPTS) {
            val timeSinceLockout = System.currentTimeMillis() - lockoutTime
            if (timeSinceLockout < LOCKOUT_DURATION_MS) {
                return true
            } else {
                // Lockout period passed, reset
                prefs.edit()
                    .putInt(PREF_FAILED_ATTEMPTS, 0)
                    .putLong(PREF_LOCKOUT_TIME, 0)
                    .apply()
            }
        }
        return false
    }

    private fun showLockedUI() {
        pinInput.isEnabled = false
        pinInput.visibility = View.GONE
        btnAuthenticate.isEnabled = false
        btnAuthenticate.text = "LOCKED"
        authMessage.text = "Device locked due to too many failed authentication attempts. Try again in 5 minutes."
        lockoutWarning.visibility = View.VISIBLE
        biometricToggle.visibility = View.GONE
        btnSetup.visibility = View.GONE
    }

    private fun showPinAuthentication() {
        if (isFirstRun) {
            // First time setup
            authMessage.text = "Welcome to GibberLink! Set up your security PIN (4 digits) and optionally enable biometric authentication."
            pinInput.hint = "Enter new PIN (4 digits)"
            btnAuthenticate.text = "Set PIN"
            btnSetup.visibility = View.GONE
            biometricContainer.visibility = View.VISIBLE
            pinCard.visibility = View.VISIBLE
        } else {
            // Normal authentication
            authMessage.text = "Enter your PIN to access GibberLink"
            pinInput.hint = "PIN (4 digits)"
            btnAuthenticate.text = "Authenticate"
            btnSetup.visibility = View.VISIBLE
            biometricContainer.visibility = View.GONE
            pinCard.visibility = View.VISIBLE
        }
        pinInput.visibility = View.VISIBLE
        pinInput.isEnabled = true
        btnAuthenticate.isEnabled = true
        btnAuthenticate.visibility = View.VISIBLE
    }

    private fun updateAuthUI() {
        // Show app information
        appTitle.text = "GibberLink"
        authMessage.text = "Securing your communication"

        // Hide progress by default
        progressBar.visibility = View.GONE
    }

    private fun onAuthenticateButtonClick() {
        val enteredPin = pinInput.text.toString()

        if (isFirstRun) {
            // Setup phase
            if (validateAndSetNewPin(enteredPin)) {
                val biometricEnabled = biometricToggle.isChecked
                prefs.edit()
                    .putBoolean(PREF_FIRST_RUN, false)
                    .putBoolean(PREF_BIOMETRIC_ENABLED, biometricEnabled)
                    .apply()

                // Proceed to main app
                startMainApp()
            }
        } else {
            // Authentication phase
            if (validatePin(enteredPin)) {
                resetFailedAttempts()
                startMainApp()
            } else {
                handleFailedPinAttempt()
            }
        }
    }

    private fun validateAndSetNewPin(pin: String): Boolean {
        // Enforce strong PIN requirements: 6-12 digits
        if (pin.length < 6 || pin.length > 12 || !pin.all { it.isDigit() }) {
            authMessage.text = "PIN must be 6-12 digits"
            return false
        }

        // Check for weak patterns
        val weakPatterns = listOf("123456", "000000", "111111", "111111111", "123456789")
        if (weakPatterns.contains(pin) || pin.toSet().size <= 2) { // All same digit or repeating
            authMessage.text = "PIN is too weak. Choose a more complex PIN"
            return false
        }

        // Check for sequential digits
        val isSequential = (0..pin.length-3).any { i ->
            val digit = pin[i].digitToInt()
            pin[i+1].digitToInt() == digit + 1 && pin[i+2].digitToInt() == digit + 2
        } || (0..pin.length-3).any { i ->
            val digit = pin[i].digitToInt()
            pin[i+1].digitToInt() == digit - 1 && pin[i+2].digitToInt() == digit - 2
        }

        if (isSequential) {
            authMessage.text = "PIN cannot contain sequential digits"
            return false
        }

        // Set the PIN
        prefs.edit().putString(PREF_APP_PIN, pin).apply()
        return true
    }

    private fun validatePin(pin: String): Boolean {
        // Updated validation to match new PIN requirements
        if (pin.length < 6 || pin.length > 12 || !pin.all { it.isDigit() }) {
            return false
        }

        val storedPin = prefs.getString(PREF_APP_PIN, null)
        return storedPin != null && storedPin == pin
    }

    private fun handleFailedPinAttempt() {
        val failedAttempts = prefs.getInt(PREF_FAILED_ATTEMPTS, 0) + 1

        prefs.edit()
            .putInt(PREF_FAILED_ATTEMPTS, failedAttempts)
            .apply()

        if (failedAttempts >= MAX_FAILED_ATTEMPTS) {
            prefs.edit()
                .putLong(PREF_LOCKOUT_TIME, System.currentTimeMillis())
                .apply()
            showLockedUI()
        } else {
            val remaining = MAX_FAILED_ATTEMPTS - failedAttempts
            authMessage.text = "Incorrect PIN. $remaining attempts remaining"
            lockoutWarning.text = "⚠️ Incorrect PIN attempts: $failedAttempts/$MAX_FAILED_ATTEMPTS"
            lockoutWarning.visibility = if (failedAttempts >= MAX_FAILED_ATTEMPTS - 1) View.VISIBLE else View.GONE
        }
    }

    private fun resetFailedAttempts() {
        prefs.edit()
            .putInt(PREF_FAILED_ATTEMPTS, 0)
            .putLong(PREF_LOCKOUT_TIME, 0)
            .apply()
    }

    private fun onBiometricSuccess() {
        resetFailedAttempts()
        startMainApp()
    }

    private fun onBiometricError(errorCode: Int, errString: String) {
        Log.w(TAG, "Biometric authentication error: $errorCode - $errString")
        when (errorCode) {
            BiometricPrompt.ERROR_NEGATIVE_BUTTON -> {
                // User tapped "Use PIN"
                showPinAuthentication()
            }
            BiometricPrompt.ERROR_LOCKOUT, BiometricPrompt.ERROR_LOCKOUT_PERMANENT -> {
                authMessage.text = "Biometric authentication locked. Use PIN authentication."
                showPinAuthentication()
            }
            else -> {
                Toast.makeText(this, "Biometric authentication failed: $errString", Toast.LENGTH_SHORT).show()
                showPinAuthentication()
            }
        }
    }

    private fun onBiometricFailed() {
        // Authentication failed but can retry
        Toast.makeText(this, "Authentication failed, please try again", Toast.LENGTH_SHORT).show()
    }

    private fun showSetupDialog() {
        // Create a setup dialog to change PIN or biometric settings
        val builder = android.app.AlertDialog.Builder(this)
        builder.setTitle("GibberLink Options")
        builder.setItems(arrayOf("Change PIN", "Biometric Settings", "Face Validation", "Range Detection")) { _, which ->
            when (which) {
                0 -> showPinChangeDialog()
                1 -> showBiometricSettingsDialog()
                2 -> showFaceValidationSettingsDialog()
                3 -> startRangeDetection()
            }
        }
        builder.show()
    }

    private fun showPinChangeDialog() {
        val input = EditText(this)
        input.inputType = android.text.InputType.TYPE_CLASS_NUMBER or android.text.InputType.TYPE_NUMBER_VARIATION_PASSWORD
        input.hint = "New 4-digit PIN"

        val builder = android.app.AlertDialog.Builder(this)
        builder.setTitle("Change PIN")
        builder.setView(input)

        builder.setPositiveButton("Update") { _, _ ->
            val newPin = input.text.toString()
            if (validateAndSetNewPin(newPin)) {
                Toast.makeText(this, "PIN updated successfully", Toast.LENGTH_SHORT).show()
                prefs.edit().putBoolean(PREF_FIRST_RUN, false).apply()
            }
        }

        builder.setNegativeButton("Cancel", null)
        builder.show()
    }

    private fun showBiometricSettingsDialog() {
        val builder = android.app.AlertDialog.Builder(this)
        builder.setTitle("Biometric Authentication")

        val biometricEnabled = prefs.getBoolean(PREF_BIOMETRIC_ENABLED, false)
        val message = if (biometricAvailable) {
            "Biometric authentication is ${if (biometricEnabled) "enabled" else "disabled"}. Would you like to ${if (biometricEnabled) "disable" else "enable"} it?"
        } else {
            "Biometric authentication is not available on this device."
        }

        builder.setMessage(message)

        if (biometricAvailable) {
            builder.setPositiveButton(if (biometricEnabled) "Disable" else "Enable") { _, _ ->
                prefs.edit()
                    .putBoolean(PREF_BIOMETRIC_ENABLED, !biometricEnabled)
                    .apply()
                Toast.makeText(this, "Biometric authentication ${if (!biometricEnabled) "enabled" else "disabled"}", Toast.LENGTH_SHORT).show()
            }
        }

        builder.setNegativeButton("Cancel", null)
        builder.show()
    }

    private fun showFaceValidationSettingsDialog() {
        val builder = android.app.AlertDialog.Builder(this)
        builder.setTitle("Face Validation")

        val faceValidationEnabled = prefs.getBoolean("face_validation_enabled", false)
        val message = "Face validation allows you to verify human presence during app usage. " +
                     "When enabled, it will periodically request face confirmation for security-critical operations.\n\n" +
                     "Status: ${if (faceValidationEnabled) "Enabled" else "Disabled"}"

        builder.setMessage(message)

        builder.setPositiveButton(if (faceValidationEnabled) "Disable" else "Enable") { _, _ ->
            prefs.edit()
                .putBoolean("face_validation_enabled", !faceValidationEnabled)
                .apply()
            Toast.makeText(this, "Face validation ${if (!faceValidationEnabled) "enabled" else "disabled"}", Toast.LENGTH_SHORT).show()
        }

        builder.setNegativeButton("Test Face Validation") { _, _ ->
            startFaceValidation()
        }

        builder.setNeutralButton("Cancel", null)
        builder.show()
    }

    private fun startFaceValidation() {
        val intent = Intent(this, FaceValidationActivity::class.java)
        faceValidationLauncher.launch(intent)
    }

    private fun startRangeDetection() {
        val intent = Intent(this, RangeDetectionActivity::class.java)
        rangeDetectionLauncher.launch(intent)
    }

    // Activity result handler for face validation
    private val faceValidationLauncher = registerForActivityResult(
        ActivityResultContracts.StartActivityForResult()
    ) { result ->
        if (result.resultCode == RESULT_OK) {
            Toast.makeText(this, "Face validation successful!", Toast.LENGTH_SHORT).show()
        } else {
            Toast.makeText(this, "Face validation failed or was cancelled", Toast.LENGTH_SHORT).show()
        }
    }

    // Activity result handler for range detection
    private val rangeDetectionLauncher = registerForActivityResult(
        ActivityResultContracts.StartActivityForResult()
    ) { result ->
        if (result.resultCode == RESULT_OK) {
            Toast.makeText(this, "Range detection completed", Toast.LENGTH_SHORT).show()
        }
    }

    private fun startMainApp() {
        // Here you would start the actual main application activity
        // For now, just show a success message
        progressBar.visibility = View.VISIBLE
        authMessage.text = "Authentication successful! Starting GibberLink..."

        // In a real implementation, you'd start the next activity
        // For demonstration, we'll just finish this activity
        progressBar.postDelayed({
            Toast.makeText(this, "Welcome to GibberLink!", Toast.LENGTH_LONG).show()
            // Intent to start the next activity would go here
            // For now, close the auth screen
            finish()
        }, 1500)
    }
}
