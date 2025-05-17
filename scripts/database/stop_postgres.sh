#!/bin/sh
set -eu

docker stop postgres_db
docker rm postgres_db
