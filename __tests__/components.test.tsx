import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { VoiceInput } from '../app/components/VoiceInput';
import { TranscriptDisplay } from '../app/components/TranscriptDisplay';
import { AgentResults } from '../app/components/AgentResults';
import { AgentSelector } from '../app/components/AgentSelector';
import { useVoiceStore } from '../app/store/voiceStore';

// Reset store before each test
beforeEach(() => {
  useVoiceStore.getState().reset();
});

describe('VoiceInput', () => {
  it('renders record button', () => {
    render(<VoiceInput />);

    const button = screen.getByRole('button', { name: /start recording/i });
    expect(button).toBeDefined();
  });

  it('shows recording state when recording', () => {
    useVoiceStore.setState({ recordingState: 'recording' });

    render(<VoiceInput />);

    const button = screen.getByRole('button', { name: /stop recording/i });
    expect(button).toBeDefined();
  });

  it('shows processing state', () => {
    useVoiceStore.setState({ recordingState: 'processing' });

    render(<VoiceInput />);

    expect(screen.getByText(/processing/i)).toBeDefined();
  });

  it('displays speech detection status when recording', () => {
    useVoiceStore.setState({
      recordingState: 'recording',
      isSpeechDetected: true
    });

    render(<VoiceInput />);

    expect(screen.getByText(/speech detected/i)).toBeDefined();
  });
});

describe('TranscriptDisplay', () => {
  it('renders nothing when no transcript and not recording', () => {
    render(<TranscriptDisplay />);

    expect(screen.queryByText(/transcript/i)).toBeNull();
  });

  it('shows transcript when available', () => {
    useVoiceStore.setState({ transcript: 'Hello world' });

    render(<TranscriptDisplay />);

    expect(screen.getByText('Hello world')).toBeDefined();
  });

  it('shows interim transcript in italic', () => {
    useVoiceStore.setState({
      transcript: 'Hello',
      interimTranscript: 'world'
    });

    render(<TranscriptDisplay />);

    expect(screen.getByText('Hello')).toBeDefined();
    expect(screen.getByText('world')).toBeDefined();
  });

  it('shows live indicator when recording', () => {
    useVoiceStore.setState({
      recordingState: 'recording',
      transcript: 'Testing'
    });

    render(<TranscriptDisplay />);

    expect(screen.getByText('Live')).toBeDefined();
  });
});

describe('AgentSelector', () => {
  it('renders all agent buttons', () => {
    render(<AgentSelector />);

    expect(screen.getByText('Action Items')).toBeDefined();
    expect(screen.getByText('Tone Shifter')).toBeDefined();
    expect(screen.getByText('Music Matcher')).toBeDefined();
  });

  it('disables buttons when no transcript', () => {
    render(<AgentSelector />);

    const buttons = screen.getAllByRole('button');
    buttons.forEach(button => {
      expect(button).toHaveProperty('disabled', true);
    });
  });

  it('enables buttons when transcript exists', () => {
    useVoiceStore.setState({ transcript: 'Some transcript text' });

    render(<AgentSelector />);

    const buttons = screen.getAllByRole('button');
    buttons.forEach(button => {
      expect(button).toHaveProperty('disabled', false);
    });
  });

  it('highlights active agent', () => {
    useVoiceStore.setState({
      transcript: 'Some text',
      activeAgent: 'action-items'
    });

    render(<AgentSelector />);

    const actionItemsButton = screen.getByText('Action Items').closest('button');
    expect(actionItemsButton?.className).toContain('bg-voice-primary');
  });
});

describe('AgentResults', () => {
  it('renders nothing when no active agent', () => {
    const { container } = render(<AgentResults />);

    expect(container.firstChild).toBeNull();
  });

  it('shows processing state', () => {
    useVoiceStore.setState({
      activeAgent: 'action-items',
      isProcessing: true,
      processingMessage: 'Extracting action items...'
    });

    render(<AgentResults />);

    expect(screen.getByText('Extracting action items...')).toBeDefined();
  });

  it('displays action items', () => {
    useVoiceStore.setState({
      activeAgent: 'action-items',
      actionItems: [
        {
          task: 'Complete the report',
          assignee: 'John',
          due_date: 'Friday',
          priority: 'high',
          context: null,
        },
      ]
    });

    render(<AgentResults />);

    expect(screen.getByText('Complete the report')).toBeDefined();
    expect(screen.getByText('@John')).toBeDefined();
  });

  it('displays tone shift result', () => {
    useVoiceStore.setState({
      activeAgent: 'tone-shifter',
      toneShiftResult: {
        original: 'Hey dude',
        shifted: 'Hello colleague',
        tone: 'professional',
      }
    });

    render(<AgentResults />);

    expect(screen.getByText('Hello colleague')).toBeDefined();
  });

  it('displays music tracks', () => {
    useVoiceStore.setState({
      activeAgent: 'music-matcher',
      musicTracks: [
        {
          id: '1',
          title: 'Test Song',
          artist: 'Test Artist',
          album: null,
          duration_ms: 180000,
          preview_url: null,
          cover_art_url: null,
          match_score: 0.9,
          mood_tags: ['happy'],
          genre_tags: ['pop'],
        },
      ],
      moodAnalysis: {
        detected_mood: 'happy',
        energy_level: 0.8,
        valence: 0.7,
        keywords: ['upbeat'],
      }
    });

    render(<AgentResults />);

    expect(screen.getByText('Test Song')).toBeDefined();
    expect(screen.getByText('Test Artist')).toBeDefined();
    expect(screen.getByText('happy')).toBeDefined();
  });
});

describe('Voice Store', () => {
  it('appends final transcript correctly', () => {
    const store = useVoiceStore.getState();

    store.appendTranscript('Hello', true);
    expect(useVoiceStore.getState().transcript).toBe('Hello');

    store.appendTranscript('world', true);
    expect(useVoiceStore.getState().transcript).toBe('Hello world');
  });

  it('sets interim transcript', () => {
    const store = useVoiceStore.getState();

    store.appendTranscript('typing...', false);
    expect(useVoiceStore.getState().interimTranscript).toBe('typing...');
  });

  it('clears transcript', () => {
    useVoiceStore.setState({
      transcript: 'Some text',
      interimTranscript: 'More text'
    });

    useVoiceStore.getState().clearTranscript();

    expect(useVoiceStore.getState().transcript).toBe('');
    expect(useVoiceStore.getState().interimTranscript).toBe('');
  });

  it('resets to initial state', () => {
    useVoiceStore.setState({
      transcript: 'Text',
      recordingState: 'recording',
      activeAgent: 'action-items',
    });

    useVoiceStore.getState().reset();

    const state = useVoiceStore.getState();
    expect(state.transcript).toBe('');
    expect(state.recordingState).toBe('idle');
    expect(state.activeAgent).toBeNull();
  });
});
