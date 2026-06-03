<script lang="ts">
  import { classAsset } from '$lib/assets';
  import { getGameClassIconId } from '$lib/data/classes';
  import type {
    MeowConnectCharacterSnapshot,
    MeowConnectLocalSnapshot,
    MeowConnectLogEntry,
    MeowConnectLogParticipant,
    MeowConnectProfile,
    MeowConnectRemoteSnapshot
  } from '$lib/services/meow-connect';

  interface LogCharacterToken {
    key: string;
    name: string;
    classId?: string;
  }

  export let logEntries: MeowConnectLogEntry[] = [];
  export let localSnapshot: MeowConnectLocalSnapshot | null = null;
  export let remoteSnapshots: MeowConnectRemoteSnapshot[] = [];
  export let currentProfile: MeowConnectProfile | null = null;

  function getClassIcon(classId: string): string {
    return getGameClassIconId(classId);
  }

  function getInitials(name: string): string {
    const parts = name.trim().split(/\s+/).filter(Boolean);
    return (parts[0]?.[0] || '?').toUpperCase() + (parts[1]?.[0] || '').toUpperCase();
  }

  function formatLogTime(timestamp: number) {
    if (!timestamp) return 'unknown';
    return new Date(timestamp).toLocaleString([], {
      month: 'short',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      hour12: false
    });
  }

  function formatLogClock(timestamp: number) {
    if (!timestamp) return 'unknown';
    return new Date(timestamp).toLocaleTimeString([], {
      hour: '2-digit',
      minute: '2-digit',
      hour12: false
    });
  }

  function formatLogTimeRange(entry: MeowConnectLogEntry) {
    const start = entry.fightStart || entry.clearedAt || 0;
    if (!start) return 'unknown';

    if (entry.source === 'Manual') {
      return formatLogTime(start);
    }

    const end = entry.clearedAt || (entry.fightStart ? entry.fightStart + Math.max(entry.duration || 0, 0) : start);
    const startDay = new Date(start).toDateString();
    const endDay = new Date(end).toDateString();

    if (startDay === endDay) {
      return `${formatLogTime(start)} - ${formatLogClock(end)}`;
    }

    return `${formatLogTime(start)} - ${formatLogTime(end)}`;
  }

  function getLogParticipants(entry: MeowConnectLogEntry): MeowConnectLogParticipant[] {
    return entry.participants?.length
      ? entry.participants
      : [{ ownerId: entry.ownerId, ownerName: entry.ownerName, ownerAvatarUrl: entry.ownerAvatarUrl, localPlayer: entry.localPlayer }];
  }

  function getVisibleLogParticipants(entry: MeowConnectLogEntry): MeowConnectLogParticipant[] {
    return getLogParticipants(entry).slice(0, 8);
  }

  function getOverflowLogParticipantCount(entry: MeowConnectLogEntry): number {
    return Math.max(getLogParticipants(entry).length - 8, 0);
  }

  function getLogAvatarStyle(index: number): string {
    return `--avatar-x: ${index % 4}; --avatar-y: ${Math.floor(index / 4)}`;
  }

  function getLogParticipantCharacters(entry: MeowConnectLogEntry): LogCharacterToken[] {
    const seen = new Set<string>();
    const characters: LogCharacterToken[] = [];

    for (const participant of getLogParticipants(entry)) {
      for (const name of splitLogCharacterNames(participant.localPlayer || participant.ownerName)) {
        const character = findLogCharacter(name, participant.ownerId);
        const displayName = character?.charName || name;
        const key = `${participant.ownerId}:${displayName.trim().toLowerCase()}`;

        if (seen.has(key)) continue;
        seen.add(key);
        characters.push({
          key,
          name: displayName,
          classId: character?.classId
        });
      }
    }

    return characters;
  }

  function splitLogCharacterNames(value: string): string[] {
    return String(value || '')
      .split(',')
      .map((name) => name.trim())
      .filter(Boolean);
  }

  function findLogCharacter(name: string, ownerId: string): MeowConnectCharacterSnapshot | undefined {
    const normalizedName = name.trim().toLowerCase();
    const localOwnerMatches = ownerId === 'local' ||
      currentProfile?.userId === ownerId ||
      currentProfile?.discordId === ownerId ||
      currentProfile?.displayName === ownerId;
    const localMatch = localOwnerMatches
      ? localSnapshot?.characters.find((character) => character.charName.trim().toLowerCase() === normalizedName)
      : undefined;

    if (localMatch) return localMatch;

    const ownerMatch = remoteSnapshots.find((snapshot) =>
      snapshot.profile.userId === ownerId ||
      snapshot.profile.discordId === ownerId ||
      snapshot.profile.displayName === ownerId
    )?.characters.find((character) => character.charName.trim().toLowerCase() === normalizedName);

    if (ownerMatch) return ownerMatch;

    return [
      ...(localSnapshot?.characters || []),
      ...remoteSnapshots.flatMap((snapshot) => snapshot.characters)
    ]
      .find((character) => character.charName.trim().toLowerCase() === normalizedName);
  }
</script>

<section class="logs-panel">
  <div class="panel-title">
    <div>
      <h3>Clear Logs</h3>
      <p>Recent MeowConnect clears from selected raids.</p>
    </div>
  </div>

  <div class="log-list">
    {#if logEntries.length === 0}
      <p class="column-empty">No shared clears for the selected raids yet.</p>
    {:else}
      {#each logEntries as entry}
        <article class="log-row">
          <div class="log-avatar-stack" style={`--avatar-count: ${getVisibleLogParticipants(entry).length}`}>
            {#each getVisibleLogParticipants(entry) as participant, participantIndex}
              {#if participant.ownerAvatarUrl}
                <img src={participant.ownerAvatarUrl} alt="" title={`${participant.ownerName} (${participant.localPlayer})`} style={getLogAvatarStyle(participantIndex)} />
              {:else}
                <span class="avatar-fallback" title={`${participant.ownerName} (${participant.localPlayer})`} style={getLogAvatarStyle(participantIndex)}>{getInitials(participant.ownerName)}</span>
              {/if}
            {/each}
            {#if getOverflowLogParticipantCount(entry) > 0}
              <span class="log-avatar-overflow">+{getOverflowLogParticipantCount(entry)}</span>
            {/if}
          </div>
          <div>
            <strong>{entry.ownerName} cleared {entry.raidName} {entry.difficulty}</strong>
            <span>
              {#if entry.gate}
                {entry.gate} |
              {/if}
              {entry.source}
              | {formatLogTimeRange(entry)}
            </span>
            <div class="log-character-line">
              {#each getLogParticipantCharacters(entry) as character (character.key)}
                <span class="log-character-token">
                  {#if character.classId}
                    <img src={classAsset(getClassIcon(character.classId))} alt="" />
                  {/if}
                  <span>{character.name}</span>
                </span>
              {/each}
            </div>
          </div>
        </article>
      {/each}
    {/if}
  </div>
</section>

<style>
  .logs-panel {
    display: grid;
    gap: 0.6rem;
    padding: 0.65rem;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 8px;
    background: var(--md-sys-color-surface);
  }

  .panel-title {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
  }

  .panel-title p,
  .column-empty {
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.82rem;
    line-height: 1.45;
  }

  .avatar-fallback {
    display: grid;
    place-items: center;
    border-radius: 50%;
    background: var(--md-sys-color-surface-variant);
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.72rem;
    font-weight: 900;
  }

  .log-list {
    display: grid;
    gap: 0.45rem;
  }

  .log-row {
    min-width: 0;
    display: grid;
    grid-template-columns: 4.55rem minmax(0, 1fr);
    gap: 0.6rem;
    align-items: start;
    padding: 0.55rem;
    border: 1px solid var(--md-sys-color-outline);
    border-radius: 8px;
    background: var(--md-sys-color-surface-variant);
  }

  .log-row > div:not(.log-avatar-stack) {
    min-width: 0;
    display: grid;
    gap: 0.15rem;
  }

  .log-row strong {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--md-sys-color-on-surface);
    font-size: 0.8rem;
  }

  .log-avatar-stack {
    --avatar-count: 1;
    position: relative;
    min-width: 2rem;
    width: 4.55rem;
    height: 3.08rem;
  }

  .log-avatar-stack img,
  .log-avatar-stack .avatar-fallback {
    position: absolute;
    top: calc(var(--avatar-y, 0) * 1.26rem);
    left: calc(var(--avatar-x, 0) * 0.82rem);
    width: 1.58rem;
    height: 1.58rem;
    border: 2px solid var(--md-sys-color-surface-variant);
    box-sizing: border-box;
  }

  .log-avatar-stack img,
  .log-character-token img {
    border-radius: 50%;
  }

  .log-avatar-overflow {
    position: absolute;
    top: 1.42rem;
    left: 3.28rem;
    display: grid;
    place-items: center;
    width: 1.2rem;
    height: 1.2rem;
    border: 2px solid var(--md-sys-color-surface-variant);
    border-radius: 50%;
    background: var(--md-sys-color-surface-container-highest);
    color: var(--md-sys-color-on-surface);
    font-size: 0.58rem;
    font-weight: 900;
    box-sizing: border-box;
  }

  .log-row span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.74rem;
  }

  .log-character-line {
    min-width: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 0.2rem 0.35rem;
    align-items: center;
    color: var(--md-sys-color-on-surface-variant);
    font-size: 0.74rem;
  }

  .log-character-token {
    display: inline-flex;
    align-items: center;
    gap: 0.18rem;
    min-width: 0;
    overflow: visible;
    white-space: nowrap;
    color: inherit;
    font-size: inherit;
  }

  .log-character-token:not(:last-child)::after {
    content: "|";
    margin-left: 0.35rem;
    color: var(--md-sys-color-outline);
  }

  .log-character-token img {
    width: 1rem;
    height: 1rem;
    flex: 0 0 1rem;
    object-fit: contain;
  }

  .log-character-token span {
    min-width: 0;
    overflow: visible;
    text-overflow: clip;
    white-space: nowrap;
    color: inherit;
    font-size: inherit;
  }

  @media (max-width: 760px) {
    .panel-title {
      display: grid;
      grid-template-columns: 1fr;
    }
  }
</style>
