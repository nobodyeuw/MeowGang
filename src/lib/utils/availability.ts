import { CALENDAR, type CalendarDay } from '../data/tasks';

export function getCurrentDayName(): string {
  const days = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];
  return days[new Date().getDay()];
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
  const currentDayIndex = now.getDay(); // 0 = Sunday, 1 = Monday, etc.
  
  // Find next available day
  let daysToAdd = 1;
  
  while (daysToAdd <= 7) {
    const nextDate = new Date(now);
    nextDate.setDate(now.getDate() + daysToAdd);
    const nextDayIndex = nextDate.getDay();
    const nextDayName = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'][nextDayIndex];
    const dayData = CALENDAR.find(day => day.day_name === nextDayName);
    
    if (dayData) {
      const isAvailable = taskId === 'gate' ? dayData.gate_available : 
                        taskId === 'boss' ? dayData.boss_available : true;
      
      if (isAvailable) {
        // Set to 10:00 AM UTC (daily reset time)
        nextDate.setUTCHours(10, 0, 0, 0);
        return nextDate;
      }
    }
    
    daysToAdd++;
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
