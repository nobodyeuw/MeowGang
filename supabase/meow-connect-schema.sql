-- MeowConnect initial Supabase schema.
-- Run this in the Supabase SQL editor for the MeowGangConnect project.
-- Favorites stay local in the LOA Tracker client and are intentionally not stored here.

create extension if not exists pgcrypto;

create table if not exists public.meow_profiles (
  user_id uuid primary key references auth.users(id) on delete cascade,
  discord_id text not null unique,
  display_name text not null,
  avatar_url text,
  consent_accepted boolean not null default false,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists public.meow_friend_connections (
  user_id uuid not null references public.meow_profiles(user_id) on delete cascade,
  friend_user_id uuid not null references public.meow_profiles(user_id) on delete cascade,
  status text not null default 'pending' check (status in ('pending', 'accepted', 'blocked')),
  shares_static boolean not null default false,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  primary key (user_id, friend_user_id),
  check (user_id <> friend_user_id)
);

alter table public.meow_friend_connections
  alter column status set default 'pending';

alter table public.meow_friend_connections
  add column if not exists shares_static boolean not null default false;

create table if not exists public.meow_characters (
  user_id uuid not null references public.meow_profiles(user_id) on delete cascade,
  char_id bigint not null,
  roster_id text not null,
  roster_name text not null,
  char_name text not null,
  class_id text not null,
  item_level numeric not null default 0,
  combat_power numeric not null default 0,
  display_order integer not null default 0,
  earns_gold boolean not null default false,
  hide_from_dashboard boolean not null default false,
  meow_connect_enabled boolean not null default false,
  has_static_reservation boolean not null default false,
  updated_at timestamptz not null default now(),
  primary key (user_id, char_id)
);

alter table public.meow_characters
  alter column meow_connect_enabled set default false;

alter table public.meow_characters
  add column if not exists has_static_reservation boolean not null default false;

create table if not exists public.meow_completion_snapshots (
  user_id uuid not null references public.meow_profiles(user_id) on delete cascade,
  roster_id text not null,
  char_id bigint not null,
  content_id text not null,
  difficulty text not null default '',
  gate text not null default 'raid',
  is_completed boolean not null default false,
  source text not null default 'manual',
  session_id text,
  reset_cycle text not null,
  completed_at bigint,
  updated_at timestamptz not null default now(),
  primary key (user_id, char_id, content_id, difficulty, gate, reset_cycle),
  foreign key (user_id, char_id) references public.meow_characters(user_id, char_id) on delete cascade
);

create table if not exists public.meow_raid_reservations (
  user_id uuid not null references public.meow_profiles(user_id) on delete cascade,
  roster_id text not null,
  char_id bigint not null,
  content_id text not null,
  difficulty text not null default '',
  reserved_for_static boolean not null default false,
  updated_at timestamptz not null default now(),
  primary key (user_id, char_id, content_id, difficulty),
  foreign key (user_id, char_id) references public.meow_characters(user_id, char_id) on delete cascade
);

create table if not exists public.meow_encounter_snapshots (
  user_id uuid not null references public.meow_profiles(user_id) on delete cascade,
  local_player text not null,
  content_id text not null,
  raid_name text not null,
  difficulty text not null default '',
  gate text not null default 'raid',
  cleared boolean not null default false,
  fight_start bigint not null,
  players_json jsonb not null default '[]'::jsonb,
  matched_character_ids_json jsonb not null default '[]'::jsonb,
  reset_cycle text not null,
  updated_at timestamptz not null default now(),
  primary key (user_id, local_player, content_id, difficulty, gate, fight_start)
);

create index if not exists idx_meow_friend_connections_friend
  on public.meow_friend_connections(friend_user_id, status);

create index if not exists idx_meow_characters_user_ilvl
  on public.meow_characters(user_id, item_level desc);

create index if not exists idx_meow_completion_lookup
  on public.meow_completion_snapshots(user_id, content_id, difficulty, reset_cycle, is_completed);

create index if not exists idx_meow_reservation_lookup
  on public.meow_raid_reservations(user_id, content_id, difficulty, reserved_for_static);

create or replace function public.meow_touch_updated_at()
returns trigger
language plpgsql
as $$
begin
  new.updated_at = now();
  return new;
end;
$$;

drop trigger if exists meow_profiles_touch_updated_at on public.meow_profiles;
create trigger meow_profiles_touch_updated_at
before update on public.meow_profiles
for each row execute function public.meow_touch_updated_at();

drop trigger if exists meow_friend_connections_touch_updated_at on public.meow_friend_connections;
create trigger meow_friend_connections_touch_updated_at
before update on public.meow_friend_connections
for each row execute function public.meow_touch_updated_at();

drop trigger if exists meow_characters_touch_updated_at on public.meow_characters;
create trigger meow_characters_touch_updated_at
before update on public.meow_characters
for each row execute function public.meow_touch_updated_at();

drop trigger if exists meow_completion_snapshots_touch_updated_at on public.meow_completion_snapshots;
create trigger meow_completion_snapshots_touch_updated_at
before update on public.meow_completion_snapshots
for each row execute function public.meow_touch_updated_at();

drop trigger if exists meow_raid_reservations_touch_updated_at on public.meow_raid_reservations;
create trigger meow_raid_reservations_touch_updated_at
before update on public.meow_raid_reservations
for each row execute function public.meow_touch_updated_at();

drop trigger if exists meow_encounter_snapshots_touch_updated_at on public.meow_encounter_snapshots;
create trigger meow_encounter_snapshots_touch_updated_at
before update on public.meow_encounter_snapshots
for each row execute function public.meow_touch_updated_at();

create or replace function public.meow_can_read_member(target_user_id uuid)
returns boolean
language sql
security definer
set search_path = public
as $$
  select target_user_id = auth.uid()
    or exists (
      select 1
      from public.meow_friend_connections c
      where c.status = 'accepted'
        and (
          (c.user_id = auth.uid() and c.friend_user_id = target_user_id)
          or (c.friend_user_id = auth.uid() and c.user_id = target_user_id)
        )
    );
$$;

create or replace function public.meow_can_read_static_member(target_user_id uuid)
returns boolean
language sql
security definer
set search_path = public
as $$
  select target_user_id = auth.uid()
    or exists (
      select 1
      from public.meow_friend_connections c
      where c.status = 'accepted'
        and c.shares_static = true
        and c.user_id = target_user_id
        and c.friend_user_id = auth.uid()
    );
$$;

create or replace function public.meow_can_read_profile(target_user_id uuid)
returns boolean
language sql
security definer
set search_path = public
as $$
  select target_user_id = auth.uid()
    or exists (
      select 1
      from public.meow_friend_connections c
      where c.status in ('pending', 'accepted')
        and (
          (c.user_id = auth.uid() and c.friend_user_id = target_user_id)
          or (c.friend_user_id = auth.uid() and c.user_id = target_user_id)
        )
    );
$$;

create or replace function public.meow_find_profile_by_discord_id(target_discord_id text)
returns table (
  user_id uuid,
  discord_id text,
  display_name text,
  avatar_url text
)
language sql
security definer
set search_path = public
as $$
  select p.user_id, p.discord_id, p.display_name, p.avatar_url
  from public.meow_profiles p
  where p.discord_id = target_discord_id
    and auth.uid() is not null
    and p.consent_accepted = true
  limit 1;
$$;

create or replace function public.meow_accept_friend_request(requester_user_id uuid)
returns void
language plpgsql
security definer
set search_path = public
as $$
begin
  if auth.uid() is null then
    raise exception 'Authentication required';
  end if;

  update public.meow_friend_connections
  set status = 'accepted'
  where user_id = requester_user_id
    and friend_user_id = auth.uid()
    and status = 'pending';
end;
$$;

create or replace function public.meow_set_static_friend(target_user_id uuid, enabled boolean)
returns void
language plpgsql
security definer
set search_path = public
as $$
begin
  if auth.uid() is null then
    raise exception 'Authentication required';
  end if;

  if not exists (
    select 1
    from public.meow_friend_connections c
    where c.status = 'accepted'
      and (
        (c.user_id = auth.uid() and c.friend_user_id = target_user_id)
        or (c.friend_user_id = auth.uid() and c.user_id = target_user_id)
      )
  ) then
    raise exception 'Accepted friend connection required';
  end if;

  insert into public.meow_friend_connections (user_id, friend_user_id, status, shares_static)
  values (auth.uid(), target_user_id, 'accepted', enabled)
  on conflict (user_id, friend_user_id)
  do update set shares_static = excluded.shares_static;
end;
$$;

alter table public.meow_profiles enable row level security;
alter table public.meow_friend_connections enable row level security;
alter table public.meow_characters enable row level security;
alter table public.meow_completion_snapshots enable row level security;
alter table public.meow_raid_reservations enable row level security;
alter table public.meow_encounter_snapshots enable row level security;

drop policy if exists "profiles readable by self and friends" on public.meow_profiles;
create policy "profiles readable by self and friends"
on public.meow_profiles for select
using (public.meow_can_read_profile(user_id));

drop policy if exists "profiles writable by owner" on public.meow_profiles;
create policy "profiles writable by owner"
on public.meow_profiles for all
using (user_id = auth.uid())
with check (user_id = auth.uid());

drop policy if exists "connections readable by participants" on public.meow_friend_connections;
create policy "connections readable by participants"
on public.meow_friend_connections for select
using (user_id = auth.uid() or friend_user_id = auth.uid());

drop policy if exists "connections writable by owner" on public.meow_friend_connections;
drop policy if exists "connections insertable by owner" on public.meow_friend_connections;
create policy "connections insertable by owner"
on public.meow_friend_connections for insert
with check (user_id = auth.uid());

drop policy if exists "connections updatable by owner" on public.meow_friend_connections;
drop policy if exists "connections updatable by participants" on public.meow_friend_connections;
create policy "connections updatable by owner"
on public.meow_friend_connections for update
using (user_id = auth.uid())
with check (user_id = auth.uid());

drop policy if exists "connections deletable by participants" on public.meow_friend_connections;
create policy "connections deletable by participants"
on public.meow_friend_connections for delete
using (user_id = auth.uid() or friend_user_id = auth.uid());

drop policy if exists "characters readable by self and friends" on public.meow_characters;
create policy "characters readable by self and friends"
on public.meow_characters for select
using (public.meow_can_read_member(user_id));

drop policy if exists "characters writable by owner" on public.meow_characters;
create policy "characters writable by owner"
on public.meow_characters for all
using (user_id = auth.uid())
with check (user_id = auth.uid());

drop policy if exists "completions readable by self and friends" on public.meow_completion_snapshots;
create policy "completions readable by self and friends"
on public.meow_completion_snapshots for select
using (public.meow_can_read_member(user_id));

drop policy if exists "completions writable by owner" on public.meow_completion_snapshots;
create policy "completions writable by owner"
on public.meow_completion_snapshots for all
using (user_id = auth.uid())
with check (user_id = auth.uid());

drop policy if exists "reservations readable by self and friends" on public.meow_raid_reservations;
create policy "reservations readable by self and friends"
on public.meow_raid_reservations for select
using (public.meow_can_read_static_member(user_id));

drop policy if exists "reservations writable by owner" on public.meow_raid_reservations;
create policy "reservations writable by owner"
on public.meow_raid_reservations for all
using (user_id = auth.uid())
with check (user_id = auth.uid());

drop policy if exists "encounters readable by self and friends" on public.meow_encounter_snapshots;
create policy "encounters readable by self and friends"
on public.meow_encounter_snapshots for select
using (public.meow_can_read_member(user_id));

drop policy if exists "encounters writable by owner" on public.meow_encounter_snapshots;
create policy "encounters writable by owner"
on public.meow_encounter_snapshots for all
using (user_id = auth.uid())
with check (user_id = auth.uid());

-- After the schema is created, enable Realtime for these tables in the Supabase dashboard:
-- public.meow_profiles
-- public.meow_friend_connections
-- public.meow_characters
-- public.meow_completion_snapshots
-- public.meow_raid_reservations
-- public.meow_encounter_snapshots
