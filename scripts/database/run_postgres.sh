#!/bin/sh
set -eu

docker run -d \
  --name postgres_db \
  -e POSTGRES_USER=rustuser \
  -e POSTGRES_PASSWORD=rustpass \
  -e POSTGRES_DB=stargazer \
  -v pgdata:/var/lib/postgresql/data \
  -p 5432:5432 \
  postgres:16
