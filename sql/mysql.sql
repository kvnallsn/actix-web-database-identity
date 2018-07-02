CREATE DATABASE twinscroll;

USE twinscroll;

CREATE TABLE identities (
	token CHAR(32) PRIMARY KEY NOT NULL,
	userid TEXT NOT NULL,
	ip TEXT,
	created DATETIME NOT NULL,
	modified DATETIME NOT NULL
);
