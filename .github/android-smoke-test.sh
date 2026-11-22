#!/usr/bin/env bash
# Install the APK on the running emulator, launch it, and check the game is
# still running a while later. Saves logcat-$API.txt and screen-$API.png.
# Run by the android-test job in .github/workflows/release.yml.
set -u
API="${API:-x}"
PKG=io.github.savvazakharov.rusty_sudoku
apk=$(ls rusty-sudoku-*-android.apk | head -n 1)

adb logcat -c
if ! adb install -r "$apk"; then
    echo "::error::APK did not install on API $API"
    exit 1
fi
adb shell am start -W -n "$PKG/.MainActivity"
sleep 25

adb exec-out screencap -p > "screen-$API.png" || true
adb logcat -d > "logcat-$API.txt"
pid=$(adb shell pidof "$PKG" | tr -d '\r')

echo "--- crash lines ---"
grep -E "FATAL EXCEPTION|UnsatisfiedLinkError|panicked|SIGSEGV|SIGABRT|Fatal signal|AndroidRuntime" "logcat-$API.txt" | head -n 40
echo "--- game output ---"
grep -iE "rusty|quad|miniquad|macroquad" "logcat-$API.txt" | head -n 60

if [ -z "$pid" ]; then
    echo "::error::The game is not running 25 s after launch on API $API"
    tail -n 80 "logcat-$API.txt"
    exit 1
fi
# A known harmless panic: on its first launch after install, Android 15
# relaunches the activity, macroquad makes a new sound context, and quad-snd's
# old audio thread panics on its now-closed message channel
# (opensles_snd.rs, `rx1.recv().unwrap()`, RecvError) as it ends. The game
# and its new audio thread carry on, so that one isn't counted as a crash;
# any other panic still is.
benign='quad-snd-[0-9.]+/src/opensles_snd\.rs'
if grep -E "panicked" "logcat-$API.txt" | grep -qE "$benign"; then
    echo "::notice::Ignoring quad-snd's old audio thread ending after the activity was relaunched (API $API)"
fi
if grep -E "FATAL EXCEPTION|panicked|Fatal signal" "logcat-$API.txt" | grep -vqE "$benign"; then
    echo "::error::The game crashed on API $API"
    exit 1
fi
echo "Game running on API $API (pid $pid)"
