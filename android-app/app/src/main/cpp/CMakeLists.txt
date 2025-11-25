cmake_minimum_required(VERSION 3.22.1)

project("gibberlink-jni")

# Import Rust library (disabled for now - library not fully implemented)
# set(RUST_LIB_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../../../../../rgibberlink-core/target/aarch64-linux-android/release")
# set(RUST_LIB_NAME "gibberlink_core")

# add_library(${RUST_LIB_NAME} STATIC IMPORTED)
# set_target_properties(${RUST_LIB_NAME} PROPERTIES IMPORTED_LOCATION
#     ${RUST_LIB_DIR}/libgibberlink_core.a)

# Find required packages for long-range hardware support
find_package(OpenCV QUIET)  # For camera processing
find_package(OpenMP QUIET)  # For parallel processing

# JNI wrapper library with long-range hardware support
add_library(gibberlink-jni SHARED
    gibberlink-jni.cpp)

target_include_directories(gibberlink-jni PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}
    ${CMAKE_CURRENT_SOURCE_DIR}/../../../../../rgibberlink-core
    ${CMAKE_CURRENT_SOURCE_DIR}/../../../../../rgibberlink-core/src)  # For Rust FFI headers

# Add OpenCV includes if available
if(OpenCV_FOUND)
    target_include_directories(gibberlink-jni PRIVATE ${OpenCV_INCLUDE_DIRS})
    target_compile_definitions(gibberlink-jni PRIVATE OPENCV_AVAILABLE)
endif()

# Add OpenMP support if available
if(OpenMP_CXX_FOUND)
    target_link_libraries(gibberlink-jni OpenMP::OpenMP_CXX)
    target_compile_definitions(gibberlink-jni PRIVATE OPENMP_AVAILABLE)
endif()

target_link_libraries(gibberlink-jni
    # ${RUST_LIB_NAME}  # Disabled - Rust library not ready
    c++_static  # C++ standard library
    c++abi      # C++ ABI for exceptions and RTTI
    unwind      # For exception handling
    android
    log
    camera2ndk  # For camera hardware access
    mediandk    # For media processing
    atomic)     # For thread-safe operations

# Enable C++17 for modern features
set_target_properties(gibberlink-jni PROPERTIES
    CXX_STANDARD 17
    CXX_STANDARD_REQUIRED ON
    CXX_EXTENSIONS OFF)

# Add compile definitions for long-range features
target_compile_definitions(gibberlink-jni PRIVATE
    LONG_RANGE_ENABLED
    PARAMETRIC_AUDIO_SUPPORT
    LASER_MODULATION_SUPPORT
    HARDWARE_SAFETY_MONITORING)

# Add optimization flags for performance
if(CMAKE_BUILD_TYPE STREQUAL "Release")
    target_compile_options(gibberlink-jni PRIVATE
        -O3
        -ffast-math
        -funroll-loops
        -fomit-frame-pointer)
endif()