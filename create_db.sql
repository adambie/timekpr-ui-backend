-- Initial database schema migration

CREATE TABLE IF NOT EXISTS settings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    key TEXT UNIQUE NOT NULL,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS managed_users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL,
    system_ip TEXT NOT NULL,
    is_valid BOOLEAN DEFAULT FALSE,
    date_added TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    last_checked TIMESTAMP,
    last_config TEXT,
    pending_time_adjustment INTEGER,
    pending_time_operation TEXT CHECK(pending_time_operation IN ('+', '-'))
);

CREATE TABLE IF NOT EXISTS user_time_usage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    date DATE NOT NULL,
    time_spent INTEGER DEFAULT 0,
    FOREIGN KEY (user_id) REFERENCES managed_users (id) ON DELETE CASCADE,
    UNIQUE(user_id, date)
);

CREATE TABLE IF NOT EXISTS user_weekly_schedule (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
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

CREATE TABLE IF NOT EXISTS user_daily_time_interval (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    day_of_week INTEGER NOT NULL CHECK(day_of_week >= 1 AND day_of_week <= 7),
    start_hour INTEGER NOT NULL CHECK(start_hour >= 0 AND start_hour <= 23),
    start_minute INTEGER DEFAULT 0 CHECK(start_minute >= 0 AND start_minute <= 59),
    end_hour INTEGER NOT NULL CHECK(end_hour >= 0 AND end_hour <= 23),
    end_minute INTEGER DEFAULT 0 CHECK(end_minute >= 0 AND end_minute <= 59),
    is_enabled BOOLEAN DEFAULT TRUE,
    is_synced BOOLEAN DEFAULT FALSE,
    last_synced TIMESTAMP,
    last_modified TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES managed_users (id) ON DELETE CASCADE,
    UNIQUE(user_id, day_of_week)
);