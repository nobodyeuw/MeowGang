export interface ChangelogChange {
  type: string;
  description: string;
  details?: string;
}

export interface ChangelogVersion {
  version: string;
  date: string;
  changes: ChangelogChange[];
  details?: string;
}

export interface ChangelogData {
  versions?: ChangelogVersion[];
}

export interface TrackerItem {
  description: string;
  details?: string;
  severity?: string;
  priority?: string;
  category?: string;
}

export interface KnownBugsData {
  knownIssues?: TrackerItem[];
  comingFeatures?: TrackerItem[];
  bugs?: TrackerItem[];
}

export interface UpdateCheckResult {
  current_version: string;
  latest_version: string | null;
  update_available: boolean;
  body?: string | null;
}
