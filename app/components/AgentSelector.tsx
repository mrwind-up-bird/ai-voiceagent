'use client';

import { useCallback } from 'react';
import { useVoiceStore, AgentType } from '../store/voiceStore';

const agents: Array<{
  id: AgentType;
  name: string;
  description: string;
  icon: React.ReactNode;
}> = [
  {
    id: 'action-items',
    name: 'Action Items',
    description: 'Extract tasks and commitments',
    icon: <ChecklistIcon />,
  },
  {
    id: 'tone-shifter',
    name: 'Tone Shifter',
    description: 'Rewrite in different tones',
    icon: <ToneIcon />,
  },
  // Music Matcher disabled - backend functions available for future implementation
  // {
  //   id: 'music-matcher',
  //   name: 'Music Matcher',
  //   description: 'Find matching music',
  //   icon: <MusicIcon />,
  // },
  {
    id: 'translator',
    name: 'Translator',
    description: 'Translate to different languages',
    icon: <TranslateIcon />,
  },
  {
    id: 'dev-log',
    name: 'Dev-Log',
    description: 'Generate commit, ticket, and update',
    icon: <DevLogIcon />,
  },
  {
    id: 'brain-dump',
    name: 'Brain Dump',
    description: 'Categorize thoughts into tasks, ideas, and notes',
    icon: <BrainIcon />,
  },
  {
    id: 'mental-mirror',
    name: 'Letter to Myself',
    description: 'Transform thoughts into a compassionate letter to future self',
    icon: <HeartIcon />,
  },
];

export function AgentSelector() {
  const { activeAgent, setActiveAgent, transcript, isProcessing, setProcessing, setError } = useVoiceStore();

  const runAgent = useCallback(
    async (agentId: AgentType) => {
      if (!agentId || !transcript || isProcessing) return;

      setActiveAgent(agentId);
      setProcessing(true, `Running ${agentId}...`);

      try {
        const { invoke } = await import('@tauri-apps/api/core');

        // Get API keys
        const openaiKey = await invoke<string | null>('get_api_key', { keyType: 'openai' });
        const anthropicKey = await invoke<string | null>('get_api_key', { keyType: 'anthropic' });

        switch (agentId) {
          case 'action-items':
            if (!openaiKey) {
              throw new Error('OpenAI API key required for Action Items');
            }
            await invoke('extract_action_items', {
              apiKey: openaiKey,
              transcript,
            });
            break;

          case 'tone-shifter':
            if (!anthropicKey) {
              throw new Error('Anthropic API key required for Tone Shifter');
            }
            const { selectedTone, toneIntensity } = useVoiceStore.getState();
            await invoke('shift_tone_streaming', {
              apiKey: anthropicKey,
              text: transcript,
              targetTone: selectedTone,
              intensity: toneIntensity,
            });
            break;

          case 'music-matcher':
            if (!openaiKey) {
              throw new Error('OpenAI API key required for Music Matcher');
            }
            // First analyze mood, then match music
            await invoke('analyze_mood_from_transcript', {
              openaiKey,
              transcript,
            });
            const qrecordsKey = await invoke<string | null>('get_api_key', { keyType: 'qrecords' });
            if (qrecordsKey) {
              await invoke('match_music', {
                apiKey: qrecordsKey,
                request: { query: transcript },
              });
            }
            break;

          case 'translator':
            if (!openaiKey) {
              throw new Error('OpenAI API key required for Translator');
            }
            const { selectedSourceLanguage, selectedTargetLanguage } = useVoiceStore.getState();
            await invoke('translate_text_streaming', {
              apiKey: openaiKey,
              text: transcript,
              sourceLanguage: selectedSourceLanguage,
              targetLanguage: selectedTargetLanguage,
            });
            break;

          case 'dev-log':
            if (!openaiKey) {
              throw new Error('OpenAI API key required for Dev-Log');
            }
            await invoke('generate_dev_log_streaming', {
              apiKey: openaiKey,
              transcript,
            });
            break;

          case 'brain-dump':
            if (!openaiKey) {
              throw new Error('OpenAI API key required for Brain Dump');
            }
            await invoke('process_brain_dump_streaming', {
              apiKey: openaiKey,
              transcript,
            });
            break;

          case 'mental-mirror':
            if (!openaiKey) {
              throw new Error('OpenAI API key required for Letter to Myself');
            }
            await invoke('generate_mental_mirror_streaming', {
              apiKey: openaiKey,
              transcript,
            });
            break;
        }
      } catch (error) {
        console.error('Agent error:', error);
        setError(error instanceof Error ? error.message : 'Agent failed');
        setProcessing(false);
      }
    },
    [transcript, isProcessing, setActiveAgent, setProcessing, setError]
  );

  return (
    <div className="flex flex-wrap justify-center gap-2">
      {agents.map((agent) => (
        <button
          key={agent.id}
          onClick={() => runAgent(agent.id)}
          disabled={!transcript || isProcessing}
          className={`
            flex items-center gap-2 px-3 py-2 rounded-lg
            text-sm font-medium transition-all duration-200
            disabled:opacity-50 disabled:cursor-not-allowed
            ${
              activeAgent === agent.id
                ? 'bg-voice-primary text-white'
                : 'bg-voice-surface text-gray-300 hover:bg-voice-border hover:text-white'
            }
          `}
          title={agent.description}
        >
          <span className="w-4 h-4">{agent.icon}</span>
          <span>{agent.name}</span>
        </button>
      ))}
    </div>
  );
}

function ChecklistIcon() {
  return (
    <svg fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4"
      />
    </svg>
  );
}

function ToneIcon() {
  return (
    <svg fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M7 8h10M7 12h4m1 8l-4-4H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-3l-4 4z"
      />
    </svg>
  );
}

function MusicIcon() {
  return (
    <svg fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3"
      />
    </svg>
  );
}

function TranslateIcon() {
  return (
    <svg fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10M12.751 5C11.783 10.77 8.07 15.61 3 18.129"
      />
    </svg>
  );
}

function DevLogIcon() {
  return (
    <svg fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4"
      />
    </svg>
  );
}

function BrainIcon() {
  return (
    <svg fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z"
      />
    </svg>
  );
}

function HeartIcon() {
  return (
    <svg fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M4.318 6.318a4.5 4.5 0 000 6.364L12 20.364l7.682-7.682a4.5 4.5 0 00-6.364-6.364L12 7.636l-1.318-1.318a4.5 4.5 0 00-6.364 0z"
      />
    </svg>
  );
}
