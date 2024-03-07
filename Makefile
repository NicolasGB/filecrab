.PHONY: up
up:
	docker-compose up -d --remove-orphans

.PHONY: down
down:
	docker-compose down --remove-orphans

.PHONY: build
build:
	docker build  . -t filecrab -f Dockerfile.dev

.PHONY: build-prod
build-prod:
	docker build  . -t filecrab


.PHONY: rebuild
rebuild: 
	make build
	make up

.PHONY: rebuild-prod
rebuild-prod: 
	make build-prod
	make up
