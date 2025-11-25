package com.Rgibberlink

import android.content.Context
import android.content.Intent
import android.graphics.Color
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.view.View
import android.widget.*
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import kotlin.concurrent.thread

class RangeDetectionActivity : AppCompatActivity() {

    companion object {
        private const val TAG = "RangeDetectionActivity"
        private const val UPDATE_INTERVAL_MS = 100L // 10Hz updates
    }

    // UI Components
    private lateinit var rangeValue: TextView
    private lateinit var signalStrength: TextView
    private lateinit var qualityScore: TextView
    private lateinit var rangeCategory: TextView
    private lateinit var statusIndicator: View
    private lateinit var btnStartStop: Button
    private lateinit var btnCalibrate: Button
    private lateinit var progressBar: ProgressBar
    private lateinit var rangeHistory: TextView

    // Range detector state
    private var rangeDetectorPtr: Long = 0
    private var isActive = false
    private var measurementHandler: Handler? = null
    private var measurementRunnable: Runnable? = null

    // Measurement history
    private val measurements = mutableListOf<Triple<Float, Float, Float>>()
    private val maxHistorySize = 10

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_range_detection)

        // Initialize UI components
        initializeViews()

        // Initialize range detector
        initializeRangeDetector()

        // Set up measurement loop
        setupMeasurementLoop()
    }

    private fun initializeViews() {
        rangeValue = findViewById(R.id.rangeValue)
        signalStrength = findViewById(R.id.signalStrength)
        qualityScore = findViewById(R.id.qualityScore)
        rangeCategory = findViewById(R.id.rangeCategory)
        statusIndicator = findViewById(R.id.statusIndicator)
        btnStartStop = findViewById(R.id.btnStartStop)
        btnCalibrate = findViewById(R.id.btnCalibrate)
        progressBar = findViewById(R.id.progressBar)
        rangeHistory = findViewById(R.id.rangeHistory)

        // Set click listeners
        btnStartStop.setOnClickListener { toggleMeasurement() }
        btnCalibrate.setOnClickListener { performCalibration() }

        // Initial UI state
        updateUI(false, 0f, 0f, 0f)
    }

    private fun initializeRangeDetector() {
        try {
            // Create range detector instance
            rangeDetectorPtr = RgibberLinkJNI.createRangeDetector()

            if (rangeDetectorPtr != 0L) {
                // Initialize the detector
                val initialized = RgibberLinkJNI.initializeRangeDetector(rangeDetectorPtr)
                if (initialized) {
                    Log.i(TAG, "Range detector initialized successfully")
                    updateStatusIndicator(true)
                } else {
                    Log.e(TAG, "Failed to initialize range detector")
                    showError("Failed to initialize range detector")
                }
            } else {
                Log.e(TAG, "Failed to create range detector")
                showError("Failed to create range detector")
            }
        } catch (e: Exception) {
            Log.e(TAG, "Error initializing range detector", e)
            showError("Error initializing range detector: ${e.message}")
        }
    }

    private fun setupMeasurementLoop() {
        measurementHandler = Handler(Looper.getMainLooper())
        measurementRunnable = object : Runnable {
            override fun run() {
                if (isActive && rangeDetectorPtr != 0L) {
                    performMeasurement()
                }
                measurementHandler?.postDelayed(this, UPDATE_INTERVAL_MS)
            }
        }
    }

    private fun toggleMeasurement() {
        isActive = !isActive

        if (isActive) {
            startMeasurement()
        } else {
            stopMeasurement()
        }
    }

    private fun startMeasurement() {
        btnStartStop.text = "Stop Measurement"
        btnStartStop.setBackgroundColor(ContextCompat.getColor(this, android.R.color.holo_red_dark))
        progressBar.visibility = View.VISIBLE

        // Start measurement loop
        measurementRunnable?.let { measurementHandler?.post(it) }

        Log.i(TAG, "Range measurement started")
    }

    private fun stopMeasurement() {
        btnStartStop.text = "Start Measurement"
        btnStartStop.setBackgroundColor(ContextCompat.getColor(this, android.R.color.holo_green_dark))
        progressBar.visibility = View.GONE

        // Stop measurement loop
        measurementRunnable?.let { measurementHandler?.removeCallbacks(it) }

        Log.i(TAG, "Range measurement stopped")
    }

    private fun performMeasurement() {
        if (rangeDetectorPtr == 0L) return

        try {
            // Prepare output arrays
            val distanceArray = FloatArray(1)
            val strengthArray = FloatArray(1)
            val qualityArray = FloatArray(1)

            // Perform measurement
            val success = RgibberLinkJNI.measureDistance(
                rangeDetectorPtr,
                distanceArray,
                strengthArray,
                qualityArray
            )

            if (success) {
                val distance = distanceArray[0]
                val strength = strengthArray[0]
                val quality = qualityArray[0]

                // Update UI
                updateUI(true, distance, strength, quality)

                // Add to history
                addToHistory(distance, strength, quality)

                Log.d(TAG, "Measurement: distance=%.2fm, strength=%.2f, quality=%.2f".format(distance, strength, quality))
            } else {
                Log.w(TAG, "Measurement failed")
                updateUI(false, 0f, 0f, 0f)
            }
        } catch (e: Exception) {
            Log.e(TAG, "Error during measurement", e)
            updateUI(false, 0f, 0f, 0f)
        }
    }

    private fun performCalibration() {
        if (rangeDetectorPtr == 0L) return

        btnCalibrate.isEnabled = false
        progressBar.visibility = View.VISIBLE

        thread {
            try {
                // Perform multiple measurements for calibration
                val calibrationSamples = 10
                var totalDistance = 0f
                var totalStrength = 0f
                var totalQuality = 0f
                var validSamples = 0

                for (i in 0 until calibrationSamples) {
                    val distanceArray = FloatArray(1)
                    val strengthArray = FloatArray(1)
                    val qualityArray = FloatArray(1)

                    val success = RgibberLinkJNI.measureDistanceAveraged(
                        rangeDetectorPtr,
                        5, // 5 samples per measurement
                        distanceArray,
                        strengthArray,
                        qualityArray
                    )

                    if (success) {
                        totalDistance += distanceArray[0]
                        totalStrength += strengthArray[0]
                        totalQuality += qualityArray[0]
                        validSamples++
                    }

                    Thread.sleep(200) // Small delay between measurements
                }

                if (validSamples > 0) {
                    val avgDistance = totalDistance / validSamples
                    val avgStrength = totalStrength / validSamples
                    val avgQuality = totalQuality / validSamples

                    runOnUiThread {
                        updateUI(true, avgDistance, avgStrength, avgQuality)
                        Toast.makeText(this, "Calibration completed", Toast.LENGTH_SHORT).show()
                    }

                    Log.i(TAG, "Calibration completed: avg_distance=%.2fm, avg_strength=%.2f, avg_quality=%.2f"
                        .format(avgDistance, avgStrength, avgQuality))
                } else {
                    runOnUiThread {
                        Toast.makeText(this, "Calibration failed - no valid measurements", Toast.LENGTH_SHORT).show()
                    }
                }
            } catch (e: Exception) {
                Log.e(TAG, "Error during calibration", e)
                runOnUiThread {
                    Toast.makeText(this, "Calibration error: ${e.message}", Toast.LENGTH_SHORT).show()
                }
            } finally {
                runOnUiThread {
                    btnCalibrate.isEnabled = true
                    progressBar.visibility = if (isActive) View.VISIBLE else View.GONE
                }
            }
        }
    }

    private fun updateUI(active: Boolean, distance: Float, strength: Float, quality: Float) {
        if (active && distance > 0) {
            rangeValue.text = "%.2f m".format(distance)
            signalStrength.text = "%.2f".format(strength)
            qualityScore.text = "%.2f".format(quality)

            // Determine range category
            val category = when {
                distance < 10 -> "Very Close"
                distance < 25 -> "Close"
                distance < 50 -> "Medium"
                distance < 100 -> "Far"
                else -> "Extreme"
            }
            rangeCategory.text = category

            // Update status indicator color based on quality
            val color = when {
                quality > 0.8 -> Color.GREEN
                quality > 0.6 -> Color.YELLOW
                quality > 0.4 -> Color.rgb(255, 165, 0) // Orange
                else -> Color.RED
            }
            statusIndicator.setBackgroundColor(color)
        } else {
            rangeValue.text = "-- m"
            signalStrength.text = "--"
            qualityScore.text = "--"
            rangeCategory.text = "No Signal"
            statusIndicator.setBackgroundColor(Color.GRAY)
        }
    }

    private fun addToHistory(distance: Float, strength: Float, quality: Float) {
        measurements.add(Triple(distance, strength, quality))

        // Keep only recent measurements
        if (measurements.size > maxHistorySize) {
            measurements.removeAt(0)
        }

        // Update history display
        val historyText = measurements.takeLast(5).joinToString("\n") { (d, s, q) ->
            "%.1fm (%.2f)".format(d, q)
        }
        rangeHistory.text = "Recent:\n$historyText"
    }

    private fun updateStatusIndicator(active: Boolean) {
        statusIndicator.setBackgroundColor(
            if (active) Color.GREEN else Color.RED
        )
    }

    private fun showError(message: String) {
        Toast.makeText(this, message, Toast.LENGTH_LONG).show()
        updateStatusIndicator(false)
    }

    override fun onDestroy() {
        super.onDestroy()

        // Stop measurements
        stopMeasurement()

        // Clean up range detector
        if (rangeDetectorPtr != 0L) {
            try {
                RgibberLinkJNI.shutdownRangeDetector(rangeDetectorPtr)
                RgibberLinkJNI.destroyRangeDetector(rangeDetectorPtr)
                Log.i(TAG, "Range detector cleaned up")
            } catch (e: Exception) {
                Log.e(TAG, "Error cleaning up range detector", e)
            }
        }
    }

    // Activity result handler for returning to main activity
    private val backToMainLauncher = registerForActivityResult(
        ActivityResultContracts.StartActivityForResult()
    ) { result ->
        if (result.resultCode == RESULT_OK) {
            finish() // Return to main activity
        }
    }
}