@echo off
"C:\\Users\\redga\\AppData\\Local\\Android\\Sdk\\cmake\\3.22.1\\bin\\cmake.exe" ^
  "-HD:\\_ APP\\RealGibber\\android-app\\app\\src\\main\\cpp" ^
  "-DCMAKE_SYSTEM_NAME=Android" ^
  "-DCMAKE_EXPORT_COMPILE_COMMANDS=ON" ^
  "-DCMAKE_SYSTEM_VERSION=24" ^
  "-DANDROID_PLATFORM=android-24" ^
  "-DANDROID_ABI=arm64-v8a" ^
  "-DCMAKE_ANDROID_ARCH_ABI=arm64-v8a" ^
  "-DANDROID_NDK=d:\\_ APP\\RealGibber\\android-ndk\\android-ndk-r27c" ^
  "-DCMAKE_ANDROID_NDK=d:\\_ APP\\RealGibber\\android-ndk\\android-ndk-r27c" ^
  "-DCMAKE_TOOLCHAIN_FILE=d:\\_ APP\\RealGibber\\android-ndk\\android-ndk-r27c\\build\\cmake\\android.toolchain.cmake" ^
  "-DCMAKE_MAKE_PROGRAM=C:\\Users\\redga\\AppData\\Local\\Android\\Sdk\\cmake\\3.22.1\\bin\\ninja.exe" ^
  "-DCMAKE_CXX_FLAGS=-std=c++17 -frtti -fexceptions" ^
  "-DCMAKE_LIBRARY_OUTPUT_DIRECTORY=D:\\_ APP\\RealGibber\\android-app\\app\\build\\intermediates\\cxx\\Debug\\5w2y5t6g\\obj\\arm64-v8a" ^
  "-DCMAKE_RUNTIME_OUTPUT_DIRECTORY=D:\\_ APP\\RealGibber\\android-app\\app\\build\\intermediates\\cxx\\Debug\\5w2y5t6g\\obj\\arm64-v8a" ^
  "-DCMAKE_BUILD_TYPE=Debug" ^
  "-BD:\\_ APP\\RealGibber\\android-app\\app\\.cxx\\Debug\\5w2y5t6g\\arm64-v8a" ^
  -GNinja
