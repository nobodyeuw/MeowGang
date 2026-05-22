import { writable } from 'svelte/store';

const SPLIT_RAT_TODO_VIEW_STORAGE_KEY = 'todo.splitRatCharacters';

function readBooleanPreference(key: string, fallback = false): boolean {
  if (typeof localStorage === 'undefined') return fallback;
  return localStorage.getItem(key) === 'true';
}

export const splitRatTodoView = writable<boolean>(
  readBooleanPreference(SPLIT_RAT_TODO_VIEW_STORAGE_KEY, false)
);

export function setSplitRatTodoView(enabled: boolean) {
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem(SPLIT_RAT_TODO_VIEW_STORAGE_KEY, String(enabled));
  }
  splitRatTodoView.set(enabled);
}

