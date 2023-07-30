CREATE TABLE word_types (
	id SERIAL PRIMARY KEY,
	english_name TEXT NOT NULL UNIQUE
);

CREATE TABLE words (
	word TEXT NOT NULL,
	word_type INTEGER REFERENCES word_types,
	language INTEGER REFERENCES languages,
	PRIMARY KEY(word, word_type, language)
);
