import { describe, it, expect, vi, beforeEach } from 'vitest';
import { invoke } from '@tauri-apps/api/core';

vi.mock('@tauri-apps/api/core');

const mockInvoke = vi.mocked(invoke);

describe('Action Items Agent', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should extract action items from transcript', async () => {
    const mockResult = {
      items: [
        {
          task: 'Schedule meeting with team',
          assignee: 'John',
          due_date: 'Friday',
          priority: 'high',
          context: 'Discussed project timeline',
        },
      ],
      summary: 'One action item identified',
    };

    mockInvoke.mockResolvedValueOnce(mockResult);

    const result = await invoke('extract_action_items', {
      apiKey: 'test-key',
      transcript: 'John, please schedule a meeting with the team by Friday.',
    });

    expect(mockInvoke).toHaveBeenCalledWith('extract_action_items', {
      apiKey: 'test-key',
      transcript: 'John, please schedule a meeting with the team by Friday.',
    });
    expect(result).toEqual(mockResult);
    expect(result.items).toHaveLength(1);
    expect(result.items[0].priority).toBe('high');
  });

  it('should handle empty transcript', async () => {
    const mockResult = { items: [], summary: 'No action items found' };
    mockInvoke.mockResolvedValueOnce(mockResult);

    const result = await invoke('extract_action_items', {
      apiKey: 'test-key',
      transcript: '',
    });

    expect(result.items).toHaveLength(0);
  });
});

describe('Tone Shifter Agent', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should shift tone to professional', async () => {
    const mockResult = {
      original: 'Hey, can you get this done ASAP?',
      shifted: 'Could you please prioritize completing this task at your earliest convenience?',
      tone: 'professional',
      suggestions: [],
    };

    mockInvoke.mockResolvedValueOnce(mockResult);

    const result = await invoke('shift_tone', {
      apiKey: 'test-key',
      text: 'Hey, can you get this done ASAP?',
      targetTone: 'professional',
    });

    expect(mockInvoke).toHaveBeenCalledWith('shift_tone', {
      apiKey: 'test-key',
      text: 'Hey, can you get this done ASAP?',
      targetTone: 'professional',
    });
    expect(result.tone).toBe('professional');
    expect(result.shifted).not.toBe(result.original);
  });

  it('should return available tones', async () => {
    const tones = [
      'professional',
      'casual',
      'friendly',
      'formal',
      'empathetic',
      'assertive',
      'diplomatic',
      'enthusiastic',
    ];

    mockInvoke.mockResolvedValueOnce(tones);

    const result = await invoke('get_available_tones');

    expect(result).toEqual(tones);
    expect(result).toContain('professional');
  });
});

describe('Music Matcher Agent', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should analyze mood from transcript', async () => {
    const mockMood = {
      detected_mood: 'energetic',
      energy_level: 0.8,
      valence: 0.7,
      keywords: ['excited', 'motivated', 'ready'],
    };

    mockInvoke.mockResolvedValueOnce(mockMood);

    const result = await invoke('analyze_mood_from_transcript', {
      openaiKey: 'test-key',
      transcript: 'I am so excited to start this new project! Let\'s go!',
    });

    expect(result.detected_mood).toBe('energetic');
    expect(result.energy_level).toBeGreaterThan(0.5);
  });

  it('should match music based on query', async () => {
    const mockResult = {
      query: 'happy upbeat music',
      tracks: [
        {
          id: 'track-1',
          title: 'Happy Song',
          artist: 'Test Artist',
          album: 'Test Album',
          duration_ms: 180000,
          preview_url: null,
          cover_art_url: null,
          match_score: 0.95,
          mood_tags: ['happy', 'upbeat'],
          genre_tags: ['pop'],
        },
      ],
      analysis: {
        detected_mood: 'happy',
        energy_level: 0.9,
        valence: 0.85,
        keywords: ['upbeat'],
      },
    };

    mockInvoke.mockResolvedValueOnce(mockResult);

    const result = await invoke('match_music', {
      apiKey: 'test-key',
      request: { query: 'happy upbeat music' },
    });

    expect(result.tracks).toHaveLength(1);
    expect(result.tracks[0].match_score).toBeGreaterThan(0.9);
  });
});

describe('Secrets Management', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should store and retrieve API keys', async () => {
    mockInvoke.mockResolvedValueOnce(undefined); // set_api_key
    mockInvoke.mockResolvedValueOnce('test-api-key'); // get_api_key

    await invoke('set_api_key', {
      keyType: 'openai',
      value: 'test-api-key',
    });

    const result = await invoke('get_api_key', {
      keyType: 'openai',
    });

    expect(result).toBe('test-api-key');
  });

  it('should check if API keys exist', async () => {
    mockInvoke.mockResolvedValueOnce(true);

    const hasKeys = await invoke('has_api_keys');

    expect(hasKeys).toBe(true);
  });
});

describe('Audio Recording', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should start recording', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);

    await invoke('start_recording');

    expect(mockInvoke).toHaveBeenCalledWith('start_recording');
  });

  it('should stop recording', async () => {
    mockInvoke.mockResolvedValueOnce(undefined);

    await invoke('stop_recording');

    expect(mockInvoke).toHaveBeenCalledWith('stop_recording');
  });

  it('should check recording state', async () => {
    mockInvoke.mockResolvedValueOnce(true);

    const isRecording = await invoke('is_recording');

    expect(isRecording).toBe(true);
  });

  it('should list audio devices', async () => {
    const devices = ['Built-in Microphone', 'External Microphone'];
    mockInvoke.mockResolvedValueOnce(devices);

    const result = await invoke('list_audio_devices');

    expect(result).toEqual(devices);
    expect(result).toHaveLength(2);
  });
});
