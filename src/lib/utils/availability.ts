import { CALENDAR, type CalendarDay } from '../data/tasks';

export function getCurrentDayName(): string {
  const days = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];
  const now = new Date();
  // Use the game day based on the 10:00 UTC daily reset boundary.
  // Before 10:00 UTC the previous day's schedule is still active.
  let dayIndex = now.getUTCDay();
  if (now.getUTCHours() < 10) {
    dayIndex = (dayIndex + 6) % 7; // previous day
  }
  return days[dayIndex];
}

export function isTaskAvailable(taskId: string): boolean {
  const currentDay = getCurrentDayName();
  const dayData = CALENDAR.find(day => day.day_name === currentDay);
  
  if (!dayData) return false;
  
  switch (taskId) {
    case 'gate':
      return dayData.gate_available;
    case 'boss':
      return dayData.boss_available;
    default:
      return true; // Non-calendar tasks are always available
  }
}

export function getNextAvailableTime(taskId: string): Date | null {
  if (isTaskAvailable(taskId)) return null;
  
  const now = new Date();
  const days = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];
  
  // Start from the next game-day boundary (10:00 UTC).
  // Before 10:00 UTC → next boundary is today 10:00 UTC.
  // After  10:00 UTC → next boundary is tomorrow 10:00 UTC.
  const nextReset = new Date(now);
  nextReset.setUTCHours(10, 0, 0, 0);
  if (now.getTime() >= nextReset.getTime()) {
    nextReset.setUTCDate(nextReset.getUTCDate() + 1);
  }
  
  for (let i = 0; i < 7; i++) {
    const checkDate = new Date(nextReset);
    checkDate.setUTCDate(nextReset.getUTCDate() + i);
    const dayName = days[checkDate.getUTCDay()];
    const dayData = CALENDAR.find(day => day.day_name === dayName);
    
    if (dayData) {
      const isAvailable = taskId === 'gate' ? dayData.gate_available :
                          taskId === 'boss' ? dayData.boss_available : true;
      if (isAvailable) {
        return checkDate;
      }
    }
  }
  
  return null;
}

export function getTimeUntilAvailable(taskId: string): string {
  const nextAvailable = getNextAvailableTime(taskId);
  
  if (!nextAvailable) return '';
  
  const now = new Date();
  const diff = nextAvailable.getTime() - now.getTime();
  
  if (diff <= 0) return '';
  
  const days = Math.floor(diff / (1000 * 60 * 60 * 24));
  const hours = Math.floor((diff % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60));
  const minutes = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60));
  
  if (days > 0) {
    return `${days}d ${hours}h ${minutes}m`;
  } else if (hours > 0) {
    return `${hours}h ${minutes}m`;
  } else {
    return `${minutes}m`;
  }
}

export function getCurrentAvailabilityStatus(): { gate: boolean; boss: boolean } {
  const currentDay = getCurrentDayName();
  const dayData = CALENDAR.find(day => day.day_name === currentDay);
  
  return {
    gate: dayData?.gate_available ?? false,
    boss: dayData?.boss_available ?? false
  };
}
