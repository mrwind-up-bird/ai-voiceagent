// Sub-Project E — selected persona id storage helpers.
//
// We keep the *choice* (which persona to apply) in localStorage rather
// than in the OS keychain because it isn't sensitive and we need
// frequent reads from React without an async invoke roundtrip. The
// underlying *token* lives in the keychain (Sub-Project D).

const PERSONA_KEY = 'aurus.persona.selectedId';
const CIRCLE_KEY = 'aurus.persona.selectedCircleId';

export interface PersonaPreference {
  personaId: string | null;
  circleId: string | null;
}

export function getPersonaPreference(): PersonaPreference {
  if (typeof window === 'undefined') {
    return { personaId: null, circleId: null };
  }
  return {
    personaId: window.localStorage.getItem(PERSONA_KEY),
    circleId: window.localStorage.getItem(CIRCLE_KEY),
  };
}

export function setPersonaPreference(pref: PersonaPreference): void {
  if (typeof window === 'undefined') return;
  if (pref.personaId) {
    window.localStorage.setItem(PERSONA_KEY, pref.personaId);
  } else {
    window.localStorage.removeItem(PERSONA_KEY);
  }
  if (pref.circleId) {
    window.localStorage.setItem(CIRCLE_KEY, pref.circleId);
  } else {
    window.localStorage.removeItem(CIRCLE_KEY);
  }
  // Custom event so other tabs / windows update — primarily helps
  // the settings page communicate with the main window if both
  // are open during dev hot-reload.
  window.dispatchEvent(new CustomEvent('aurus:persona-changed'));
}

export function clearPersonaPreference(): void {
  setPersonaPreference({ personaId: null, circleId: null });
}
