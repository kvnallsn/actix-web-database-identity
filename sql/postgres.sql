CREATE DATABASE twinscroll;

\c twinscroll

CREATE TABLE identities (
	token TEXT PRIMARY KEY NOT NULL,
	userid TEXT NOT NULL
);
