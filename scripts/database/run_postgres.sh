#!/bin/sh
set -eu

CONTAINER_NAME="postgres_db"

# Check if container exists
if docker ps -a --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
    # Container exists, check if it's running
    if docker ps --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
        echo "Container ${CONTAINER_NAME} is already running"
        exit 0
    else
        echo "Starting existing container ${CONTAINER_NAME}"
        docker start ${CONTAINER_NAME}
        exit 0
    fi
fi

echo "Creating new container ${CONTAINER_NAME}"
docker run -d \
  --name ${CONTAINER_NAME} \
  -e POSTGRES_USER=rustuser \
  -e POSTGRES_PASSWORD=rustpass \
  -e POSTGRES_DB=stargazer \
  -v pgdata:/var/lib/postgresql/data \
  -p 5432:5432 \
  postgres:16
