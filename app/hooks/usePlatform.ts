'use client';

import { useState, useEffect } from 'react';

export type Platform = 'desktop' | 'ios' | 'android' | 'web';

interface PlatformInfo {
  platform: Platform;
  isDesktop: boolean;
  isMobile: boolean;
  isIOS: boolean;
  isAndroid: boolean;
  isTauri: boolean;
  supportsKeyboardShortcuts: boolean;
  supportsWindowControls: boolean;
}

/**
 * Detects the current platform (desktop/iOS/Android/web).
 * Used to conditionally render platform-specific UI elements.
 */
export function usePlatform(): PlatformInfo {
  const [platformInfo, setPlatformInfo] = useState<PlatformInfo>({
    platform: 'web',
    isDesktop: false,
    isMobile: false,
    isIOS: false,
    isAndroid: false,
    isTauri: false,
    supportsKeyboardShortcuts: true,
    supportsWindowControls: false,
  });

  useEffect(() => {
    async function detectPlatform() {
      let platform: Platform = 'web';
      let isTauri = false;

      // Check if running in Tauri
      try {
        const { type, platform: osPlatform } = await import('@tauri-apps/plugin-os');
        const osType = await type();
        const os = await osPlatform();
        isTauri = true;

        if (osType === 'ios') {
          platform = 'ios';
        } else if (osType === 'android' || os === 'android') {
          platform = 'android';
        } else {
          platform = 'desktop';
        }
      } catch {
        // Not in Tauri, detect via user agent
        const ua = navigator.userAgent.toLowerCase();
        if (/iphone|ipad|ipod/.test(ua)) {
          platform = 'ios';
        } else if (/android/.test(ua)) {
          platform = 'android';
        } else if (/mobile/.test(ua)) {
          platform = 'android'; // Generic mobile
        } else {
          platform = 'web';
        }
      }

      const isDesktop = platform === 'desktop';
      const isMobile = platform === 'ios' || platform === 'android';
      const isIOS = platform === 'ios';
      const isAndroid = platform === 'android';

      setPlatformInfo({
        platform,
        isDesktop,
        isMobile,
        isIOS,
        isAndroid,
        isTauri,
        supportsKeyboardShortcuts: isDesktop,
        supportsWindowControls: isDesktop && isTauri,
      });
    }

    detectPlatform();
  }, []);

  return platformInfo;
}

/**
 * Hook to check if we're running in Tauri (any platform).
 */
export function useIsTauri(): boolean {
  const [isTauri, setIsTauri] = useState(false);

  useEffect(() => {
    async function check() {
      try {
        await import('@tauri-apps/api/core');
        setIsTauri(true);
      } catch {
        setIsTauri(false);
      }
    }
    check();
  }, []);

  return isTauri;
}
