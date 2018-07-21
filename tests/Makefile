MAKEFILE	:= $(lastword $(MAKEFILE_LIST))

include test.env

.PHONY: help all clean reset

help:
	@echo "Actix Web SQL Identity Test Management\n"
	@echo "Targets\n-------"
	@echo "help:           Display this message"
	@echo "all:            Initialize sqlite, postgres and mysql databases"
	@echo "clean:          Deletes/drops sqlite, postgres and mysql databases and tables"
	@echo "reset:          clean, followed by all"
	@echo ""
	@echo "sqlite:         Initialize sqlite database"
	@echo "pg:             Initialize posgtresql database"
	@echo "mysql:          Initialize mysql database"
	@echo ""
	@echo "clean_sqlite:   Removes all sqlite database files"
	@echo "clean_pg:       Drops configured database and tables for postgres" 
	@echo "clean_mysql:    Drops configured database and tables for mysql"
	@echo ""
	@echo "reset_sqlite:   Drops sqlite database/tabels then recreates them"
	@echo "reset_pg:       Drops pg database/tables then recreates them"
	@echo "reset_mysql:    Drops mysql database/tables then creates them"

all: sqlite pg mysql

clean: clean_sqlite clean_pg clean_mysql

reset: reset_sqlite reset_pg reset_mysql

.PHONY: create_sqlite
sqlite:
	sqlite3 ${SQLITE_DB} < ../sql/sqlite.sql
	sqlite3 ${SQLITE_DB2} < ../sql/sqlite.sql
	sqlite3 ${SQLITE_DB3} < ../sql/sqlite.sql
	sqlite3 ${SQLITE_DB} < load.sql
	sqlite3 ${SQLITE_DB2} < load.sql
	sqlite3 ${SQLITE_DB3} < load.sql

.PHONY: create_pg
pg:
	psql -f ../sql/postgres.sql -U ${PG_USER}
	psql -f load.sql -U ${PG_USER} -d ${PG_DB}

.PHONY: create_mysql
mysql:
	mysql -u ${MYSQL_USER} -p"${MYSQL_PASS}" < ../sql/mysql.sql
	mysql -u ${MYSQL_USER} -p"${MYSQL_PASS}" -D ${MYSQL_DB} < load.sql

.PHONY: clean_sqlite
clean_sqlite:
	rm -f ${SQLITE_DB} ${SQLITE_DB2} ${SQLITE_DB3}

.PHONY: clean_pg
clean_pg:
	psql -c "DROP DATABASE IF EXISTS ${PG_DB};" -U ${PG_USER}

.PHONY: clean_mysql
clean_mysql:
	mysql -e "DROP DATABASE IF EXISTS ${MYSQL_DB};" -u ${MYSQL_USER} -p"${MYSQL_PASS}"

.PHONY: reset_sqlite
reset_sqlite:
	@$(MAKE) -f $(MAKEFILE) clean_sqlite
	@$(MAKE) -f $(MAKEFILE) sqlite

.PHONY: reset_pg
reset_pg:
	@$(MAKE) -f $(MAKEFILE) clean_pg
	@$(MAKE) -f $(MAKEFILE) pg

.PHONY: reset_mysql
reset_mysql:
	@$(MAKE) -f $(MAKEFILE) clean_mysql
	@$(MAKE) -f $(MAKEFILE) mysql
