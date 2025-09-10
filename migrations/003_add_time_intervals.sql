-- Add time interval columns to user_weekly_schedule table
-- This extends the existing schedule table with start and end times for each day

ALTER TABLE user_weekly_schedule ADD COLUMN monday_start_hour INTEGER DEFAULT 0 CHECK(monday_start_hour >= 0 AND monday_start_hour <= 23);
ALTER TABLE user_weekly_schedule ADD COLUMN monday_start_minute INTEGER DEFAULT 0 CHECK(monday_start_minute >= 0 AND monday_start_minute <= 59);
ALTER TABLE user_weekly_schedule ADD COLUMN monday_end_hour INTEGER DEFAULT 23 CHECK(monday_end_hour >= 0 AND monday_end_hour <= 23);
ALTER TABLE user_weekly_schedule ADD COLUMN monday_end_minute INTEGER DEFAULT 59 CHECK(monday_end_minute >= 0 AND monday_end_minute <= 59);

ALTER TABLE user_weekly_schedule ADD COLUMN tuesday_start_hour INTEGER DEFAULT 0 CHECK(tuesday_start_hour >= 0 AND tuesday_start_hour <= 23);
ALTER TABLE user_weekly_schedule ADD COLUMN tuesday_start_minute INTEGER DEFAULT 0 CHECK(tuesday_start_minute >= 0 AND tuesday_start_minute <= 59);
ALTER TABLE user_weekly_schedule ADD COLUMN tuesday_end_hour INTEGER DEFAULT 23 CHECK(tuesday_end_hour >= 0 AND tuesday_end_hour <= 23);
ALTER TABLE user_weekly_schedule ADD COLUMN tuesday_end_minute INTEGER DEFAULT 59 CHECK(tuesday_end_minute >= 0 AND tuesday_end_minute <= 59);

ALTER TABLE user_weekly_schedule ADD COLUMN wednesday_start_hour INTEGER DEFAULT 0 CHECK(wednesday_start_hour >= 0 AND wednesday_start_hour <= 23);
ALTER TABLE user_weekly_schedule ADD COLUMN wednesday_start_minute INTEGER DEFAULT 0 CHECK(wednesday_start_minute >= 0 AND wednesday_start_minute <= 59);
ALTER TABLE user_weekly_schedule ADD COLUMN wednesday_end_hour INTEGER DEFAULT 23 CHECK(wednesday_end_hour >= 0 AND wednesday_end_hour <= 23);
ALTER TABLE user_weekly_schedule ADD COLUMN wednesday_end_minute INTEGER DEFAULT 59 CHECK(wednesday_end_minute >= 0 AND wednesday_end_minute <= 59);

ALTER TABLE user_weekly_schedule ADD COLUMN thursday_start_hour INTEGER DEFAULT 0 CHECK(thursday_start_hour >= 0 AND thursday_start_hour <= 23);
ALTER TABLE user_weekly_schedule ADD COLUMN thursday_start_minute INTEGER DEFAULT 0 CHECK(thursday_start_minute >= 0 AND thursday_start_minute <= 59);
ALTER TABLE user_weekly_schedule ADD COLUMN thursday_end_hour INTEGER DEFAULT 23 CHECK(thursday_end_hour >= 0 AND thursday_end_hour <= 23);
ALTER TABLE user_weekly_schedule ADD COLUMN thursday_end_minute INTEGER DEFAULT 59 CHECK(thursday_end_minute >= 0 AND thursday_end_minute <= 59);

ALTER TABLE user_weekly_schedule ADD COLUMN friday_start_hour INTEGER DEFAULT 0 CHECK(friday_start_hour >= 0 AND friday_start_hour <= 23);
ALTER TABLE user_weekly_schedule ADD COLUMN friday_start_minute INTEGER DEFAULT 0 CHECK(friday_start_minute >= 0 AND friday_start_minute <= 59);
ALTER TABLE user_weekly_schedule ADD COLUMN friday_end_hour INTEGER DEFAULT 23 CHECK(friday_end_hour >= 0 AND friday_end_hour <= 23);
ALTER TABLE user_weekly_schedule ADD COLUMN friday_end_minute INTEGER DEFAULT 59 CHECK(friday_end_minute >= 0 AND friday_end_minute <= 59);

ALTER TABLE user_weekly_schedule ADD COLUMN saturday_start_hour INTEGER DEFAULT 0 CHECK(saturday_start_hour >= 0 AND saturday_start_hour <= 23);
ALTER TABLE user_weekly_schedule ADD COLUMN saturday_start_minute INTEGER DEFAULT 0 CHECK(saturday_start_minute >= 0 AND saturday_start_minute <= 59);
ALTER TABLE user_weekly_schedule ADD COLUMN saturday_end_hour INTEGER DEFAULT 23 CHECK(saturday_end_hour >= 0 AND saturday_end_hour <= 23);
ALTER TABLE user_weekly_schedule ADD COLUMN saturday_end_minute INTEGER DEFAULT 59 CHECK(saturday_end_minute >= 0 AND saturday_end_minute <= 59);

ALTER TABLE user_weekly_schedule ADD COLUMN sunday_start_hour INTEGER DEFAULT 0 CHECK(sunday_start_hour >= 0 AND sunday_start_hour <= 23);
ALTER TABLE user_weekly_schedule ADD COLUMN sunday_start_minute INTEGER DEFAULT 0 CHECK(sunday_start_minute >= 0 AND sunday_start_minute <= 59);
ALTER TABLE user_weekly_schedule ADD COLUMN sunday_end_hour INTEGER DEFAULT 23 CHECK(sunday_end_hour >= 0 AND sunday_end_hour <= 23);
ALTER TABLE user_weekly_schedule ADD COLUMN sunday_end_minute INTEGER DEFAULT 59 CHECK(sunday_end_minute >= 0 AND sunday_end_minute <= 59);