-- Fix user_weekly_schedule table to have unique constraint on user_id
-- This migration recreates the table with proper unique constraint

-- Create new table with unique constraint
CREATE TABLE user_weekly_schedule_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL UNIQUE,
    monday_hours REAL DEFAULT 0,
    tuesday_hours REAL DEFAULT 0,
    wednesday_hours REAL DEFAULT 0,
    thursday_hours REAL DEFAULT 0,
    friday_hours REAL DEFAULT 0,
    saturday_hours REAL DEFAULT 0,
    sunday_hours REAL DEFAULT 0,
    is_synced BOOLEAN DEFAULT FALSE,
    last_synced TIMESTAMP,
    last_modified TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES managed_users (id) ON DELETE CASCADE
);

-- Copy latest record per user_id from old table to new table
INSERT INTO user_weekly_schedule_new (user_id, monday_hours, tuesday_hours, wednesday_hours, thursday_hours, friday_hours, saturday_hours, sunday_hours, is_synced, last_synced, last_modified)
SELECT 
    user_id,
    monday_hours,
    tuesday_hours, 
    wednesday_hours,
    thursday_hours,
    friday_hours,
    saturday_hours,
    sunday_hours,
    is_synced,
    last_synced,
    last_modified
FROM user_weekly_schedule s1
WHERE s1.id = (
    SELECT s2.id FROM user_weekly_schedule s2 
    WHERE s2.user_id = s1.user_id 
    ORDER BY s2.last_modified DESC 
    LIMIT 1
);

-- Drop old table
DROP TABLE user_weekly_schedule;

-- Rename new table
ALTER TABLE user_weekly_schedule_new RENAME TO user_weekly_schedule;