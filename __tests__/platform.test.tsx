import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, waitFor, act } from '@testing-library/react';

// --- usePlatform tests ---

// We need to re-import per test since the mock changes
let usePlatformModule: typeof import('../app/hooks/usePlatform');

describe('usePlatform', () => {
  beforeEach(() => {
    vi.resetModules();
  });

  describe('Tauri desktop detection', () => {
    it('detects desktop when Tauri OS plugin returns macos', async () => {
      vi.doMock('@tauri-apps/plugin-os', () => ({
        type: vi.fn().mockResolvedValue('macos'),
        platform: vi.fn().mockResolvedValue('macos'),
      }));

      usePlatformModule = await import('../app/hooks/usePlatform');
      const { result } = renderHook(() => usePlatformModule.usePlatform());

      await waitFor(() => {
        expect(result.current.platform).toBe('desktop');
      });

      expect(result.current.isDesktop).toBe(true);
      expect(result.current.isMobile).toBe(false);
      expect(result.current.isTauri).toBe(true);
      expect(result.current.supportsKeyboardShortcuts).toBe(true);
      expect(result.current.supportsWindowControls).toBe(true);
    });

    it('detects desktop when Tauri OS plugin returns windows', async () => {
      vi.doMock('@tauri-apps/plugin-os', () => ({
        type: vi.fn().mockResolvedValue('windows'),
        platform: vi.fn().mockResolvedValue('win32'),
      }));

      usePlatformModule = await import('../app/hooks/usePlatform');
      const { result } = renderHook(() => usePlatformModule.usePlatform());

      await waitFor(() => {
        expect(result.current.platform).toBe('desktop');
      });

      expect(result.current.isDesktop).toBe(true);
      expect(result.current.isTauri).toBe(true);
    });
  });

  describe('Tauri mobile detection', () => {
    it('detects iOS when Tauri OS type is ios', async () => {
      vi.doMock('@tauri-apps/plugin-os', () => ({
        type: vi.fn().mockResolvedValue('ios'),
        platform: vi.fn().mockResolvedValue('ios'),
      }));

      usePlatformModule = await import('../app/hooks/usePlatform');
      const { result } = renderHook(() => usePlatformModule.usePlatform());

      await waitFor(() => {
        expect(result.current.platform).toBe('ios');
      });

      expect(result.current.isIOS).toBe(true);
      expect(result.current.isMobile).toBe(true);
      expect(result.current.isDesktop).toBe(false);
      expect(result.current.isTauri).toBe(true);
      expect(result.current.supportsKeyboardShortcuts).toBe(false);
      expect(result.current.supportsWindowControls).toBe(false);
    });

    it('detects Android when Tauri OS type is android', async () => {
      vi.doMock('@tauri-apps/plugin-os', () => ({
        type: vi.fn().mockResolvedValue('android'),
        platform: vi.fn().mockResolvedValue('android'),
      }));

      usePlatformModule = await import('../app/hooks/usePlatform');
      const { result } = renderHook(() => usePlatformModule.usePlatform());

      await waitFor(() => {
        expect(result.current.platform).toBe('android');
      });

      expect(result.current.isAndroid).toBe(true);
      expect(result.current.isMobile).toBe(true);
      expect(result.current.isDesktop).toBe(false);
      expect(result.current.isTauri).toBe(true);
    });

    it('detects Android via platform fallback when type is linux', async () => {
      vi.doMock('@tauri-apps/plugin-os', () => ({
        type: vi.fn().mockResolvedValue('linux'),
        platform: vi.fn().mockResolvedValue('android'),
      }));

      usePlatformModule = await import('../app/hooks/usePlatform');
      const { result } = renderHook(() => usePlatformModule.usePlatform());

      await waitFor(() => {
        expect(result.current.platform).toBe('android');
      });

      expect(result.current.isAndroid).toBe(true);
      expect(result.current.isMobile).toBe(true);
    });
  });

  describe('user agent fallback (no Tauri)', () => {
    it('detects iOS from user agent when Tauri is unavailable', async () => {
      vi.doMock('@tauri-apps/plugin-os', () => {
        throw new Error('Module not found');
      });

      const originalUA = navigator.userAgent;
      Object.defineProperty(navigator, 'userAgent', {
        value: 'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X)',
        configurable: true,
      });

      usePlatformModule = await import('../app/hooks/usePlatform');
      const { result } = renderHook(() => usePlatformModule.usePlatform());

      await waitFor(() => {
        expect(result.current.platform).toBe('ios');
      });

      expect(result.current.isIOS).toBe(true);
      expect(result.current.isMobile).toBe(true);
      expect(result.current.isTauri).toBe(false);

      Object.defineProperty(navigator, 'userAgent', {
        value: originalUA,
        configurable: true,
      });
    });

    it('detects Android from user agent when Tauri is unavailable', async () => {
      vi.doMock('@tauri-apps/plugin-os', () => {
        throw new Error('Module not found');
      });

      const originalUA = navigator.userAgent;
      Object.defineProperty(navigator, 'userAgent', {
        value: 'Mozilla/5.0 (Linux; Android 14; Pixel 8)',
        configurable: true,
      });

      usePlatformModule = await import('../app/hooks/usePlatform');
      const { result } = renderHook(() => usePlatformModule.usePlatform());

      await waitFor(() => {
        expect(result.current.platform).toBe('android');
      });

      expect(result.current.isAndroid).toBe(true);
      expect(result.current.isMobile).toBe(true);
      expect(result.current.isTauri).toBe(false);

      Object.defineProperty(navigator, 'userAgent', {
        value: originalUA,
        configurable: true,
      });
    });

    it('defaults to web when no mobile indicators', async () => {
      vi.doMock('@tauri-apps/plugin-os', () => {
        throw new Error('Module not found');
      });

      const originalUA = navigator.userAgent;
      Object.defineProperty(navigator, 'userAgent', {
        value: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36',
        configurable: true,
      });

      usePlatformModule = await import('../app/hooks/usePlatform');
      const { result } = renderHook(() => usePlatformModule.usePlatform());

      await waitFor(() => {
        expect(result.current.platform).toBe('web');
      });

      expect(result.current.isDesktop).toBe(false);
      expect(result.current.isMobile).toBe(false);
      expect(result.current.isTauri).toBe(false);

      Object.defineProperty(navigator, 'userAgent', {
        value: originalUA,
        configurable: true,
      });
    });
  });

  describe('initial state', () => {
    it('returns web defaults before detection completes', async () => {
      // Slow Tauri mock to observe initial state
      vi.doMock('@tauri-apps/plugin-os', () => ({
        type: vi.fn().mockImplementation(() => new Promise(() => {})), // never resolves
        platform: vi.fn().mockImplementation(() => new Promise(() => {})),
      }));

      usePlatformModule = await import('../app/hooks/usePlatform');
      const { result } = renderHook(() => usePlatformModule.usePlatform());

      // Initial state before async detection
      expect(result.current.platform).toBe('web');
      expect(result.current.isDesktop).toBe(false);
      expect(result.current.isMobile).toBe(false);
      expect(result.current.isTauri).toBe(false);
    });
  });
});

// --- useIsTauri tests ---

describe('useIsTauri', () => {
  beforeEach(() => {
    vi.resetModules();
  });

  it('returns true when @tauri-apps/api/core is available', async () => {
    vi.doMock('@tauri-apps/api/core', () => ({
      invoke: vi.fn(),
    }));

    const { useIsTauri } = await import('../app/hooks/usePlatform');
    const { result } = renderHook(() => useIsTauri());

    await waitFor(() => {
      expect(result.current).toBe(true);
    });
  });

  it('returns false when @tauri-apps/api/core is unavailable', async () => {
    vi.doMock('@tauri-apps/api/core', () => {
      throw new Error('Module not found');
    });

    const { useIsTauri } = await import('../app/hooks/usePlatform');
    const { result } = renderHook(() => useIsTauri());

    // Should stay false
    await new Promise((r) => setTimeout(r, 50));
    expect(result.current).toBe(false);
  });
});

// --- useLocalAIAvailable tests ---

describe('useLocalAIAvailable', () => {
  beforeEach(() => {
    vi.resetModules();
  });

  afterEach(() => {
    delete (globalThis as any).Worker;
  });

  it('returns true when Worker and WebAssembly are available', async () => {
    // jsdom doesn't define Worker, so we mock it
    (globalThis as any).Worker = class MockWorker {};

    vi.doMock('../app/workers/ai-worker', () => ({}));
    const { useLocalAIAvailable } = await import('../app/hooks/useLocalAI');
    const { result } = renderHook(() => useLocalAIAvailable());

    await waitFor(() => {
      expect(result.current).toBe(true);
    });
  });

  it('returns false when Worker is not available', async () => {
    // Ensure Worker is not defined
    delete (globalThis as any).Worker;

    vi.doMock('../app/workers/ai-worker', () => ({}));
    const { useLocalAIAvailable } = await import('../app/hooks/useLocalAI');
    const { result } = renderHook(() => useLocalAIAvailable());

    // Should remain false
    await new Promise((r) => setTimeout(r, 50));
    expect(result.current).toBe(false);
  });
});

// --- useWebAudioCapture tests ---

describe('useWebAudioCapture', () => {
  let mockGetUserMedia: ReturnType<typeof vi.fn>;
  let mockAudioContext: {
    createMediaStreamSource: ReturnType<typeof vi.fn>;
    createScriptProcessor: ReturnType<typeof vi.fn>;
    destination: {};
    close: ReturnType<typeof vi.fn>;
  };
  let mockProcessor: {
    connect: ReturnType<typeof vi.fn>;
    disconnect: ReturnType<typeof vi.fn>;
    onaudioprocess: ((event: unknown) => void) | null;
  };
  let mockSource: { connect: ReturnType<typeof vi.fn> };
  let mockTrack: { stop: ReturnType<typeof vi.fn> };

  beforeEach(() => {
    vi.resetModules();

    mockTrack = { stop: vi.fn() };
    mockGetUserMedia = vi.fn().mockResolvedValue({
      getTracks: () => [mockTrack],
    });

    mockProcessor = {
      connect: vi.fn(),
      disconnect: vi.fn(),
      onaudioprocess: null,
    };

    mockSource = { connect: vi.fn() };

    mockAudioContext = {
      createMediaStreamSource: vi.fn().mockReturnValue(mockSource),
      createScriptProcessor: vi.fn().mockReturnValue(mockProcessor),
      destination: {},
      close: vi.fn(),
    };

    // Mock AudioContext globally
    (globalThis as any).AudioContext = vi.fn().mockImplementation(() => mockAudioContext);

    // Mock navigator.mediaDevices
    Object.defineProperty(navigator, 'mediaDevices', {
      value: {
        getUserMedia: mockGetUserMedia,
      },
      configurable: true,
    });
  });

  afterEach(() => {
    delete (globalThis as any).AudioContext;
  });

  it('detects Web Audio API support', async () => {
    // Mock usePlatform to return mobile
    vi.doMock('../app/hooks/usePlatform', () => ({
      usePlatform: () => ({
        platform: 'ios',
        isDesktop: false,
        isMobile: true,
        isIOS: true,
        isAndroid: false,
        isTauri: true,
        supportsKeyboardShortcuts: false,
        supportsWindowControls: false,
      }),
    }));
    vi.doMock('@tauri-apps/api/core', () => ({
      invoke: vi.fn(),
    }));

    const { useWebAudioCapture } = await import('../app/hooks/useWebAudioCapture');
    const { result } = renderHook(() => useWebAudioCapture());

    await waitFor(() => {
      expect(result.current.isSupported).toBe(true);
    });
  });

  it('blocks recording on desktop platform', async () => {
    vi.doMock('../app/hooks/usePlatform', () => ({
      usePlatform: () => ({
        platform: 'desktop',
        isDesktop: true,
        isMobile: false,
        isIOS: false,
        isAndroid: false,
        isTauri: true,
        supportsKeyboardShortcuts: true,
        supportsWindowControls: true,
      }),
    }));
    vi.doMock('@tauri-apps/api/core', () => ({
      invoke: vi.fn(),
    }));

    const { useWebAudioCapture } = await import('../app/hooks/useWebAudioCapture');
    const { result } = renderHook(() => useWebAudioCapture());

    await act(async () => {
      await result.current.startRecording();
    });

    expect(result.current.isRecording).toBe(false);
    expect(result.current.error).toBe('Use native audio capture on desktop');
  });

  it('requests microphone permission', async () => {
    vi.doMock('../app/hooks/usePlatform', () => ({
      usePlatform: () => ({
        platform: 'ios',
        isDesktop: false,
        isMobile: true,
        isIOS: true,
        isAndroid: false,
        isTauri: true,
        supportsKeyboardShortcuts: false,
        supportsWindowControls: false,
      }),
    }));
    vi.doMock('@tauri-apps/api/core', () => ({
      invoke: vi.fn(),
    }));

    const { useWebAudioCapture } = await import('../app/hooks/useWebAudioCapture');
    const { result } = renderHook(() => useWebAudioCapture());

    let granted: boolean = false;
    await act(async () => {
      granted = await result.current.requestPermission();
    });

    expect(granted).toBe(true);
    expect(mockGetUserMedia).toHaveBeenCalledWith({ audio: true });
    expect(mockTrack.stop).toHaveBeenCalled(); // Tracks stopped after permission check
    expect(result.current.permissionState).toBe('granted');
  });

  it('handles permission denial gracefully', async () => {
    mockGetUserMedia.mockRejectedValueOnce(new Error('NotAllowedError'));

    vi.doMock('../app/hooks/usePlatform', () => ({
      usePlatform: () => ({
        platform: 'android',
        isDesktop: false,
        isMobile: true,
        isIOS: false,
        isAndroid: true,
        isTauri: true,
        supportsKeyboardShortcuts: false,
        supportsWindowControls: false,
      }),
    }));
    vi.doMock('@tauri-apps/api/core', () => ({
      invoke: vi.fn(),
    }));

    const { useWebAudioCapture } = await import('../app/hooks/useWebAudioCapture');
    const { result } = renderHook(() => useWebAudioCapture());

    let granted: boolean = true;
    await act(async () => {
      granted = await result.current.requestPermission();
    });

    expect(granted).toBe(false);
    expect(result.current.permissionState).toBe('denied');
    expect(result.current.error).toContain('Microphone permission denied');
  });

  it('starts and stops recording on mobile', async () => {
    const mockInvoke = vi.fn().mockResolvedValue(undefined);

    vi.doMock('../app/hooks/usePlatform', () => ({
      usePlatform: () => ({
        platform: 'ios',
        isDesktop: false,
        isMobile: true,
        isIOS: true,
        isAndroid: false,
        isTauri: true,
        supportsKeyboardShortcuts: false,
        supportsWindowControls: false,
      }),
    }));
    vi.doMock('@tauri-apps/api/core', () => ({
      invoke: mockInvoke,
    }));

    const { useWebAudioCapture } = await import('../app/hooks/useWebAudioCapture');
    const { result } = renderHook(() => useWebAudioCapture());

    // Wait for support detection
    await waitFor(() => {
      expect(result.current.isSupported).toBe(true);
    });

    // Start recording
    await act(async () => {
      await result.current.startRecording();
    });

    expect(result.current.isRecording).toBe(true);
    expect(result.current.error).toBeNull();
    expect(mockGetUserMedia).toHaveBeenCalled();

    // Stop recording
    act(() => {
      result.current.stopRecording();
    });

    expect(result.current.isRecording).toBe(false);
    expect(mockTrack.stop).toHaveBeenCalled();
    expect(mockAudioContext.close).toHaveBeenCalled();
  });

  it('sends audio chunks to Rust backend via invoke', async () => {
    const mockInvoke = vi.fn().mockResolvedValue(undefined);

    vi.doMock('../app/hooks/usePlatform', () => ({
      usePlatform: () => ({
        platform: 'ios',
        isDesktop: false,
        isMobile: true,
        isIOS: true,
        isAndroid: false,
        isTauri: true,
        supportsKeyboardShortcuts: false,
        supportsWindowControls: false,
      }),
    }));
    vi.doMock('@tauri-apps/api/core', () => ({
      invoke: mockInvoke,
    }));

    const { useWebAudioCapture } = await import('../app/hooks/useWebAudioCapture');
    const { result } = renderHook(() => useWebAudioCapture());

    await waitFor(() => {
      expect(result.current.isSupported).toBe(true);
    });

    await act(async () => {
      await result.current.startRecording();
    });

    // Simulate audio data arriving via ScriptProcessor
    const fakeAudioData = new Float32Array(1600).fill(0.5); // 100ms chunk at 16kHz
    const fakeEvent = {
      inputBuffer: {
        getChannelData: () => fakeAudioData,
      },
    };

    await act(async () => {
      // Trigger the onaudioprocess callback
      mockProcessor.onaudioprocess?.(fakeEvent);
      // Allow the async invoke to process
      await new Promise((r) => setTimeout(r, 10));
    });

    expect(mockInvoke).toHaveBeenCalledWith('send_audio_to_deepgram', {
      samples: expect.any(Array),
    });
  });
});
