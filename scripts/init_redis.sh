#!/usr/bin/env bash

set -x 
set -eo pipefail

RUNNING_CONTAINER=$(docker ps --filter 'name=redis' --format '{{.ID}}')
if [[ -n $RUNNING_CONTAINER ]]; then
    echo>&2 "there is already a reddis container running"
    echo>&2 "kill it with docker kill ${RUNING_CONTAINER}"
    exit 1
fi

docker run \
    -p "6379:6379" \
    -d \
    --name "redis_$(date '+%s')" \
    redis:6
>&2 echo "redis is ready to go"