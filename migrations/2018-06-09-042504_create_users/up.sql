CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE users (
	id uuid DEFAULT uuid_generate_v4() PRIMARY KEY,
	display_name TEXT NOT NULL,
	google_id TEXT NULL UNIQUE,
	twitter_screen_name TEXT NULL UNIQUE
);