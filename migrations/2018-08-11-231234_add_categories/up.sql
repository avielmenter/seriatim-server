CREATE TABLE categories (
	id uuid DEFAULT uuid_generate_v4() PRIMARY KEY,
	user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
	document_id uuid NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
	category_name TEXT NOT NULL,
	UNIQUE(category_name, user_id, document_id)
);