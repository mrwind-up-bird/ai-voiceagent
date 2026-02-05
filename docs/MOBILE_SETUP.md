# Aurus Mobile Setup Guide

This guide covers setting up the development environment for iOS and Android builds.

## Prerequisites

### iOS Development

1. **Xcode** (macOS only)
   ```bash
   xcode-select --install
   ```

2. **Apple Developer Account**
   - Sign up at https://developer.apple.com
   - Note your Team ID (10-character alphanumeric)

3. **Configure Tauri**
   ```bash
   # Add to src-tauri/tauri.conf.json under "bundle":
   "iOS": {
     "developmentTeam": "YOUR_TEAM_ID"
   }
   ```

4. **Initialize iOS Project**
   ```bash
   pnpm tauri ios init
   ```

### Android Development

1. **Android Studio**
   - Download from https://developer.android.com/studio
   - Install Android SDK (API 26+)
   - Install NDK (side by side)

2. **Environment Variables**
   ```bash
   # Add to ~/.zshrc or ~/.bashrc
   export ANDROID_HOME="$HOME/Library/Android/sdk"
   export NDK_HOME="$ANDROID_HOME/ndk/$(ls -1 $ANDROID_HOME/ndk | tail -1)"
   export PATH="$PATH:$ANDROID_HOME/platform-tools"
   ```

3. **Configure Tauri**
   ```bash
   # Add to src-tauri/tauri.conf.json under "bundle":
   "android": {
     "minSdkVersion": 26
   }
   ```

4. **Initialize Android Project**
   ```bash
   pnpm tauri android init
   ```

## Build Commands

### iOS

```bash
# Development build
pnpm tauri ios dev

# Release build (requires signing)
pnpm tauri ios build
```

### Android

```bash
# Development build
pnpm tauri android dev

# Release APK
pnpm tauri android build

# Release AAB (for Play Store)
pnpm tauri android build --aab
```

## Mobile-Specific Permissions

### iOS (Info.plist)

The following permissions are already configured in `src-tauri/Info.plist`:

```xml
<key>NSMicrophoneUsageDescription</key>
<string>Aurus needs microphone access for voice transcription.</string>
```

For additional permissions, add to `Info.plist`:

```xml
<!-- Speech Recognition (optional, for on-device STT) -->
<key>NSSpeechRecognitionUsageDescription</key>
<string>Aurus uses speech recognition for transcription.</string>

<!-- Background Audio (for long recordings) -->
<key>UIBackgroundModes</key>
<array>
    <string>audio</string>
</array>
```

### Android (AndroidManifest.xml)

After `pnpm tauri android init`, add to `src-tauri/gen/android/app/src/main/AndroidManifest.xml`:

```xml
<uses-permission android:name="android.permission.RECORD_AUDIO" />
<uses-permission android:name="android.permission.INTERNET" />

<!-- Optional: For secure storage with biometrics -->
<uses-permission android:name="android.permission.USE_BIOMETRIC" />
```

## Architecture Notes

### Platform-Specific Code

The Rust backend uses conditional compilation:

| Module | Desktop | Mobile |
|--------|---------|--------|
| `audio.rs` | CPAL | Not available (needs native bridge) |
| `tts.rs` | Shell commands | Not available (needs native bridge) |
| `secrets.rs` | OS Keychain | iOS Keychain / Android Keystore |
| `transcription.rs` (Whisper) | whisper-rs | Not available |
| `transcription.rs` (Deepgram) | WebSocket | WebSocket |
| All agents | HTTP (reqwest) | HTTP (reqwest) |

### Capabilities

- Desktop capabilities: `src-tauri/capabilities/default.json`
- Mobile capabilities: `src-tauri/capabilities/mobile.json`

### Known Limitations (Mobile)

1. **Audio Capture**: CPAL doesn't support iOS/Android. Future work needed for native audio bridges.
2. **Local Whisper**: whisper-rs requires native compilation. Mobile uses cloud transcription only.
3. **TTS**: Shell commands not available. Needs AVSpeechSynthesizer (iOS) / TextToSpeech (Android) bridges.
4. **Global Shortcuts**: Not applicable on mobile (no system-wide hotkeys).

## Troubleshooting

### iOS: "Development team not found"
- Ensure `developmentTeam` in `tauri.conf.json` matches your Apple Developer Team ID
- Sign in to Xcode with your Apple ID: Xcode → Preferences → Accounts

### Android: "SDK not found"
- Verify `ANDROID_HOME` points to your SDK installation
- Run `source ~/.zshrc` after adding environment variables

### Build fails with "whisper-rs" errors on mobile
- This is expected. whisper-rs is desktop-only.
- The conditional compilation should exclude it. If not, check `Cargo.toml` target configs.
