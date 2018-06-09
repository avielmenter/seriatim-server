CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE users (
	user_id uuid DEFAULT uuid_generate_v4() PRIMARY KEY,
	twitter_name TEXT NULL,
	twitter_screen_name TEXT NULL
);