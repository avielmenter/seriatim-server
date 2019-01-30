CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TYPE style_unit AS ENUM ('cm', 'mm', 'in', 'px', 'pt', 'pc', 'em', 'ex', 'ch', 'rem', 'vw', 'vh', 'vmin', 'vmax', '%');
CREATE TYPE style_property AS ENUM ('background_color', 'color', 'font_size', 'line_height');

CREATE TABLE styles (
	item_id uuid NOT NULL REFERENCES items(id) ON DELETE CASCADE,
	property style_property NOT NULL,
	value_number INT NULL,
	value_string TEXT NULL,
	unit style_unit NULL,
	PRIMARY KEY(item_id, property)
);