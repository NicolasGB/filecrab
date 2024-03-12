# List all the available recipes
[private]
default:
  @just --list --unsorted

# Build the filecrab Docker image
build mode="debug":
  docker build  . -t filecrab --build-arg BUILD_MODE={{mode}}

# Build and up
rebuild mode="debug":
  just build mode={{mode}}
  just up

# Run the multi-container application
up:
  docker-compose up -d --remove-orphans

# Stop the multi-container application
down:
  docker-compose down --remove-orphans

# Show the logs of a container
logs container="filecrab":
  docker-compose logs -f -n 100 {{container}}
