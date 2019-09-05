#!/bin/bash
# Init script for loading the database into postgres.

parent_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )

psql -U tg -h localhost -c "CREATE DATABASE dellstore2 OWNER tg TEMPLATE template0;"

psql -U tg -h localhost -c "grant all privileges on database dellstore2 to tg;"

psql dellstore2 < "${parent_path}/dellstore2-normal-1.0/dellstore2-normal-1.0.sql" -U tg -h localhost