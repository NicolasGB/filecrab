# List all the available recipes
[private]
default:
  @just --list --unsorted

# Build the filecrab Docker image
build features="" mode="debug":
  docker build  . -t filecrab --build-arg BUILD_MODE={{mode}} --build-arg FEATURES={{features}}

# Build and up
rebuild features="" mode="debug":
  just build \"{{features}}\" {{mode}}
  just up

build-front:
 docker build -f Dockerfile.front . -t filecrab-front

# Run the multi-container application
up:
  docker-compose up -d --remove-orphans

# Stop the multi-container application
down:
  docker-compose down --remove-orphans

# Show the logs of a container
logs container="filecrab":
  docker-compose logs -f -n 100 {{container}}
