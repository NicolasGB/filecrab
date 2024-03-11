# Docker helper commands

mode = debug

.PHONY: default
default:
	@echo "Usage: make [COMMAND]"
	@echo
	@echo "Commands:"
	@echo "  build    Build the filecrab Docker image"
	@echo "  up       Run the multi-container application"
	@echo "  down     Stop the multi-container application"
	@echo "  rebuild  Build and up"
	@echo "  logs     Show the logs of the filecrab container"

.PHONY: build
build:
	docker build  . -t filecrab --build-arg BUILD_MODE=$(mode)

.PHONY: up
up:
	docker-compose up -d --remove-orphans

.PHONY: down
down:
	docker-compose down --remove-orphans

.PHONY: rebuild
rebuild: 
	make build
	make up

.PHONY: logs
logs:
	docker-compose logs -f filecrab
