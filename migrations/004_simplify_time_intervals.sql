-- Replace separate hour/minute columns with simple TIME columns
-- This simplifies the API and database schema

-- Drop the complex hour/minute columns
ALTER TABLE user_weekly_schedule DROP COLUMN monday_start_hour;
ALTER TABLE user_weekly_schedule DROP COLUMN monday_start_minute;
ALTER TABLE user_weekly_schedule DROP COLUMN monday_end_hour;
ALTER TABLE user_weekly_schedule DROP COLUMN monday_end_minute;

ALTER TABLE user_weekly_schedule DROP COLUMN tuesday_start_hour;
ALTER TABLE user_weekly_schedule DROP COLUMN tuesday_start_minute;
ALTER TABLE user_weekly_schedule DROP COLUMN tuesday_end_hour;
ALTER TABLE user_weekly_schedule DROP COLUMN tuesday_end_minute;

ALTER TABLE user_weekly_schedule DROP COLUMN wednesday_start_hour;
ALTER TABLE user_weekly_schedule DROP COLUMN wednesday_start_minute;
ALTER TABLE user_weekly_schedule DROP COLUMN wednesday_end_hour;
ALTER TABLE user_weekly_schedule DROP COLUMN wednesday_end_minute;

ALTER TABLE user_weekly_schedule DROP COLUMN thursday_start_hour;
ALTER TABLE user_weekly_schedule DROP COLUMN thursday_start_minute;
ALTER TABLE user_weekly_schedule DROP COLUMN thursday_end_hour;
ALTER TABLE user_weekly_schedule DROP COLUMN thursday_end_minute;

ALTER TABLE user_weekly_schedule DROP COLUMN friday_start_hour;
ALTER TABLE user_weekly_schedule DROP COLUMN friday_start_minute;
ALTER TABLE user_weekly_schedule DROP COLUMN friday_end_hour;
ALTER TABLE user_weekly_schedule DROP COLUMN friday_end_minute;

ALTER TABLE user_weekly_schedule DROP COLUMN saturday_start_hour;
ALTER TABLE user_weekly_schedule DROP COLUMN saturday_start_minute;
ALTER TABLE user_weekly_schedule DROP COLUMN saturday_end_hour;
ALTER TABLE user_weekly_schedule DROP COLUMN saturday_end_minute;

ALTER TABLE user_weekly_schedule DROP COLUMN sunday_start_hour;
ALTER TABLE user_weekly_schedule DROP COLUMN sunday_start_minute;
ALTER TABLE user_weekly_schedule DROP COLUMN sunday_end_hour;
ALTER TABLE user_weekly_schedule DROP COLUMN sunday_end_minute;

-- Add simple TIME columns with defaults (00:00 to 23:59)
ALTER TABLE user_weekly_schedule ADD COLUMN monday_start_time TEXT DEFAULT '00:00';
ALTER TABLE user_weekly_schedule ADD COLUMN monday_end_time TEXT DEFAULT '23:59';

ALTER TABLE user_weekly_schedule ADD COLUMN tuesday_start_time TEXT DEFAULT '00:00';
ALTER TABLE user_weekly_schedule ADD COLUMN tuesday_end_time TEXT DEFAULT '23:59';

ALTER TABLE user_weekly_schedule ADD COLUMN wednesday_start_time TEXT DEFAULT '00:00';
ALTER TABLE user_weekly_schedule ADD COLUMN wednesday_end_time TEXT DEFAULT '23:59';

ALTER TABLE user_weekly_schedule ADD COLUMN thursday_start_time TEXT DEFAULT '00:00';
ALTER TABLE user_weekly_schedule ADD COLUMN thursday_end_time TEXT DEFAULT '23:59';

ALTER TABLE user_weekly_schedule ADD COLUMN friday_start_time TEXT DEFAULT '00:00';
ALTER TABLE user_weekly_schedule ADD COLUMN friday_end_time TEXT DEFAULT '23:59';

ALTER TABLE user_weekly_schedule ADD COLUMN saturday_start_time TEXT DEFAULT '00:00';
ALTER TABLE user_weekly_schedule ADD COLUMN saturday_end_time TEXT DEFAULT '23:59';

ALTER TABLE user_weekly_schedule ADD COLUMN sunday_start_time TEXT DEFAULT '00:00';
ALTER TABLE user_weekly_schedule ADD COLUMN sunday_end_time TEXT DEFAULT '23:59';