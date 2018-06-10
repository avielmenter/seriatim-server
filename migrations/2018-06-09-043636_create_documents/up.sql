CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE documents (
	id uuid DEFAULT uuid_generate_v4() PRIMARY KEY,
	user_id uuid NOT NULL REFERENCES users,
	root_item_id uuid NULL,
	created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	modified_at TIMESTAMP NULL
);

CREATE TABLE items (
	id uuid DEFAULT uuid_generate_v4() PRIMARY KEY,
	document_id uuid NOT NULL REFERENCES documents,
	parent_id uuid NULL REFERENCES items,
	item_text TEXT NOT NULL,
	collapsed BOOLEAN NOT NULL DEFAULT FALSE
);

ALTER TABLE documents
	ADD CONSTRAINT fk_root_item_id
	FOREIGN KEY (root_item_id) REFERENCES items(id);