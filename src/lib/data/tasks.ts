export interface GameTask {
  id: string;          // content_id
  name: string;        // content_name 
  category: 'roster' | 'character';
  reset_schedule: 'daily' | 'weekly';
  logic_type: 'normal' | 'calendar' | 'rested';
  max_rest_value?: number; // max_rest_value
  min_ilvl?: number;    // min_ilvl requirement
}

export const GAME_TASKS: Record<string, GameTask> = {
  "gate": {
    id: "gate",
    name: "Chaos Gate",
    category: "roster",
    reset_schedule: "daily",
    logic_type: "calendar"
  },
  "boss": {
    id: "boss",
    name: "Field Boss",
    category: "roster",
    reset_schedule: "daily",
    logic_type: "calendar"
  },
  "chaos": {
    id: "chaos",
    name: "Chaos Dungeon",
    category: "character",
    reset_schedule: "daily",
    logic_type: "rested",
    max_rest_value: 100,
    min_ilvl: 1385
  },
  "guardian": {
    id: "guardian",
    name: "Guardian Raid",
    category: "character",
    reset_schedule: "daily",
    logic_type: "rested",
    max_rest_value: 100,
    min_ilvl: 1385
  },
  "cube": {
    id: "cube",
    name: "Cube",
    category: "character",
    reset_schedule: "weekly",
    logic_type: "normal"
  },
  "paradise": {
    id: "paradise",
    name: "Paradise",
    category: "character",
    reset_schedule: "weekly",
    logic_type: "normal"
  },
  "shop": {
    id: "shop",
    name: "Solo Shop",
    category: "character",
    reset_schedule: "weekly",
    logic_type: "normal"
  },
  "guild": {
    id: "guild",
    name: "Guild Shop",
    category: "character",
    reset_schedule: "weekly",
    logic_type: "normal"
  }
};

// Calendar data for availability - only applies to gate and boss tasks
export interface CalendarDay {
  day_name: string;
  gate_available: boolean;
  boss_available: boolean;
}

export const CALENDAR: CalendarDay[] = [
  { day_name: "Monday", gate_available: true, boss_available: false },
  { day_name: "Tuesday", gate_available: false, boss_available: true },
  { day_name: "Wednesday", gate_available: false, boss_available: false },
  { day_name: "Thursday", gate_available: true, boss_available: false },
  { day_name: "Friday", gate_available: false, boss_available: true },
  { day_name: "Saturday", gate_available: true, boss_available: false },
  { day_name: "Sunday", gate_available: true, boss_available: true }
];

// Helper functions
export function getTaskById(id: string): GameTask | undefined {
  return GAME_TASKS[id];
}

export function getTasksByCategory(category: 'roster' | 'character'): GameTask[] {
  return Object.keys(GAME_TASKS).map(key => GAME_TASKS[key]).filter(task => task.category === category);
}

export function getDailyTasks(): GameTask[] {
  return Object.keys(GAME_TASKS).map(key => GAME_TASKS[key]).filter(task => task.resetSchedule === 'daily');
}

export function getWeeklyTasks(): GameTask[] {
  return Object.keys(GAME_TASKS).map(key => GAME_TASKS[key]).filter(task => task.resetSchedule === 'weekly');
}

export function getRosterTasks(): GameTask[] {
  return getTasksByCategory('roster');
}

export function getCharacterTasks(): GameTask[] {
  return getTasksByCategory('character');
}

export function getAvailableDays(): CalendarDay[] {
  return CALENDAR;
}

export function getDayByName(dayName: string): CalendarDay | undefined {
  return CALENDAR.find(day => day.dayName === dayName);
}

export function isGateAvailable(dayName: string): boolean {
  const day = getDayByName(dayName);
  return day ? day.gateAvailable : false;
}

export function isBossAvailable(dayName: string): boolean {
  const day = getDayByName(dayName);
  return day ? day.bossAvailable : false;
}
