#!/usr/bin/env bash

set +x
set -eo pipefail

# Check for dependencies before launching the database
if ! [ -x "$(command -v psql)" ]; then
	echo >&2 "Error: psql is not installed"
	exit 1
fi
if ! [ -x "$(command -v sqlx)" ]; then
	>&2 echo "Error: sqlx is not installed"
	>&2 echo "Use:"
	>&2 echo "cargo install --version='~0.6' sqlx-cli --no-default-features --features rustls,postgres"
	>&2 echo "to install it."
	exit 1
fi

# Check if a custom user has been set, otherwise then to default 'postgres'
DB_USER=${POSTGRES_USER:=postgres}
# Check if a custom password has been set, otherwise then to default 'password'
DB_PASSWORD="${POSTGRES_PASSWORD:=password}"
# Check if a custom name has been set, otherwise then to default 'newsletter'
DB_NAME="${POSTGRES_NAME:=newsletter}"
# Check if a custom port has been set, otherwise then to default '5432'
DB_PORT="${POSTGRES_PORT:=5432}"
# Check if a custom host has been set, otherwise then to default 'localhost'
DB_HOST="${POSTGRES_HOST:=localhost}"

# Allow to skip if a dockerized Postgres database instance is already running
if [[ -z "${SKIP_DOCKER}" ]]
then
	# Launch postgres using Docker
	docker run \
		-e POSTGRES_USER=${DB_USER} \
		-e POSTGRES_PASSWORD=${DB_PASSWORD} \
		-e POSTGRES_DB=${DB_NAME} \
		-p "${DB_PORT}":5432 \
		-d postgres \
		postgres -N 1000
	# the above increases the maximum number of connections for testing purposes
fi

# Keep ping postgers until it's ready to accept commands
export PGPASSWORD="${DB_PASSWORD}"

readonly timeout=45
iter=1
until psql -h "${DB_HOST}" -U "${DB_USER}" -p "${DB_PORT}" -d "postgres" -c '\q'; do
	if [ "${iter}" -gt "${timeout}" ]; then
		>&2 echo  "Unable to connect to Postgres, aborting due to ${timeout}s timeout!"
		exit 1
	fi
	>&2 echo  "Postgres is still unavaliable - sleeping..."
	sleep 1
	((++iter))
done


>&2 echo  "Postgres is up and running on port: $DB_PORT}"

DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}

export DATABASE_URL

sqlx database create
sqlx migrate run


>&2 echo  "Postgres has been migrated, ready to go"
