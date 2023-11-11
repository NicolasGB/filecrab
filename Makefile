.PHONY: up
up:
	docker-compose up -d --remove-orphans

.PHONY: down
down:
	docker-compose down --remove-orphans

.PHONY: build
build:
	docker build  . -t filecrab
