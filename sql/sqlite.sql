CREATE TABLE identities (
	token TEXT PRIMARY KEY NOT NULL,
	userid TEXT NOT NULL,
	ip TEXT,
	useragent TEXT,
	created DATETIME NOT NULL,
	modified DATETIME NOT NULL
);
