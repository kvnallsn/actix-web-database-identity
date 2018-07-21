CREATE DATABASE testdb;

\c testdb

CREATE TABLE identities (
	id BIGSERIAL PRIMARY KEY NOT NULL,
	token TEXT UNIQUE NOT NULL,
	userid TEXT NOT NULL,
	ip TEXT,
	useragent TEXT,
	created timestamp NOT NULL,
	modified timestamp NOT NULL
);
