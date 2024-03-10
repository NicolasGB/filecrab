# Docker helper commands
mode = debug

## Run the multi-container application
.PHONY: up
up:
	docker-compose up -d --remove-orphans

## Stop the multi-container application
.PHONY: down
down:
	docker-compose down --remove-orphans

## Build the filecrab Docker image
.PHONY: build
build:
	docker build  . -t filecrab --build-arg BUILD_MODE=$(mode)

## Build and up
.PHONY: rebuild
rebuild: 
	make build
	make up

## Show the logs of the filecrab container
.PHONY: logs
logs:
	docker-compose logs -f filecrab
