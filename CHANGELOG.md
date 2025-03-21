# Changelog

All notable changes to this project will be documented in this file.

## [filecrab-web-v0.4.0, filecrab-sever-v0.4.0, filecrab-cli-v0.4.0] - 2025-03-21

### 🚀 Features

- _(server)_ [**breaking**] Update to "Any" engine in surreal db

### 🐛 Bug Fixes

- _(web,cli,server)_ Bump everything to edition 2024 and add wayland support for cli
- _(server)_ Update surrealdb to v2 and fix all related breakings

## [0.3.1] - 2024-11-22

### 🐛 Bug Fixes

- _(all)_ Update deps, refactor code to fix the breaking sin age

## [filecrab-web-v0.1.1] - 2024-09-23

### 🐛 Bug Fixes

- _(web)_ Use port 8080 instead of 80

## [0.3.0] - 2024-09-16

### 🚀 Features

- _(cli)_ Add styles
- _(cli)_ Detect encrypted files and prompt while execution for encryption and decryption
- _(web)_ Init frontend-web

### 🐛 Bug Fixes

- Use as ref when needed
- Cli now downlaods correctly and finished the bar
- Update query to use simple = instead of ==
- _(deps)_ Update deps and code to match new tower layering
- _(web)_ Implement donwloading and decrypting assets
- _(web)_ Read while decrypting manually, remove middleware for downloading files
- _(web)_ Add front image build, add traefick to the docker compose
- Update deps

### 💼 Other

- Test new tar release
- Add github action for building the front end
- Split ci in different tags for more granular builds

## [0.2.0] - 2024-05-23

### 🚀 Features

- _(cli)_ Handle multiple instances of filecrabs
- _(cli)_ Add a command to `add` instances to a filecrab config
- _(cli)_ Add a command to `remove` an exisiting instance from a filecrab config
- _(cli)_ Add init command, to initialize filecrab's config without running anything

### 🐛 Bug Fixes

- _(cli)_ Improve copied text for commands, it now includes the command to retreive the text/file instead of only the `memorable_word_list`

## [0.1.1] - 2024-04-22

### 🐛 Bug Fixes

- _(readme)_ Add aur information
- _(deps)_ Update deps
- _(cli)_ Handle upload errors gracefully and update deps
- _(server)_ Mark filecrab key header as sensitive
- _(changelog)_ Add changelog with git cliff

## [0.1.0] - 2024-03-27

### 🚀 Features

- _(filecrab)_ Implement upload and download
- _(docker, routes)_ Add dockerfile and dockercompose, add routes middleware
- _(docker)_ Use multistage build and cargo chef
- _(asset)_ Add creating an asset with and without password
- _(upload)_ Stream from multipart to s3
- _(download)_ Stream download
- _(middleware)_ Implement simple api key based middleware
- _(cli)_ Init cli project
- _(clipboard)_ Add copying upload response to clipboard
- _(asset)_ Create memorizable word list and use it when saving assets
- _(cli)_ Add downloading file
- _(text)_ Implement text copy and paste handler with age text encryption
- _(cli)_ Add copy and paste commands
- _(cli)_ Add init config if not set
- _(cli)_ Decrypt correctly ty @mmalecot
- _(server)_ Add server wrapped in the cli, add delete old asset
- _(server)_ In the clean command correctly delete from the minio too
- _(server)_ Start migration to thiserror
- _(cli)_ Paste can now read piped data
- _(just)_ Swith to just instead of make
- _(cli)_ Replace anyhow with thiserror
- _(server)_ Allow chosing the engine
- _(server)_ Update docker to match new conditional compilation requirements
- _(cli)_ Add inquire for user prompts. Bump dependencies
- _(cd)_ Add release actions
- _(cd)_ Add publishing docker image

### 🐛 Bug Fixes

- _(error)_ Avoid panicking creating response
- Add .env.example
- _(asset)_ Download and search correctly via surrealdb
- _(asset)_ Send conent length from file
- _(cli)_ Fix typo
- _(dockerfile, typos)_ Use alpine for slimmer final image and fix typos
- Remove unnecessary comment
- Build only server in docker image and add lto flag for release binaries
- _(server)_ Remove anyhow from server, handle statsus codes
- _(cli)_ Add spinners
- _(docker)_ Add multiple build envs
- _(server)_ Rename server package
- _(pipeline)_ Updates pipeline
- _(pipeline)_ Updates pipeline
- _(server)_ Stop storing password and remove argon2
- _(cli)_ Remove sending password to filecrab
- _(server)_ Handle gracefully http errors from s3, they are now propagated to the API's response
- _(server)_ Update code to non deprecated functions to chrono
- _(pipeline)_ Use cache with Cargo
- _(pipeline)_ Fix pipeline
- _(pipeline)_ Updates actions/cache
- _(server,cli)_ Remove server side encryption of the text, add client side encryption, delete text after retrieval
- _(pipeline)_ Move format to a dedicated job
- _(pipeline)_ Rename pipeline to ci
- _(docker)_ Use only one Dockerfile and improve docker-compose
- _(docker)_ Add usage by default on make
- _(docker)_ Add more release flags to filecrab
- _(docs)_ Adds detailed README
- _(readme)_ Update README
- _(badges)_ Fix CI badge
- _(.env.example)_ Document the fields
- _(cli)_ Rename binary build, use option instead of default and better pattern matching
- Remove trailing whitspace in match
- _(just)_ Fix typo
- _(cli)_ Add optional out file when copying text
- _(cli)_ Try to open file even before requesting the server
- _(cli)_ Improve comment
- _(cli)_ Safely open and delete file to make sure user made no typo in command
- _(cli)_ Switch to raw identifier
- _(cli)_ Replace wrong message
- _(cd)_ Update token name
- _(cd)_ Update asset_name
- _(ci,cd)_ Use dtolnay's rust action
- _(cd)_ Try better usage for meta action
- _(cd)_ Update tag
- _(readme)_ Improve CLI docs
- _(readme)_ Add TOC
- _(readme)_ Use correct headline for help
- _(readme, config)_ Update config names and imporve example and readme of the server
- _(readme)_ Add more detailed info
- _(server)_ Add text cleaning
- _(readme)_ Add correct TOC
- _(server)_ Improve connection error message
- _(server)_ Create a job that runs the cleanup on a separated green thread
- _(server)_ Remove non needed ARC and remove Clap
- _(readme)_ Update accroding new feature
- _(justfile)_ Fix empty param management
- _(cargo)_ Update package info
- _(server)_ Remove unused expire field
- _(cd)_ Update to fixed version

### 🚜 Refactor

- _(model_manager, surrealdb)_ Stop unwrapping and implement good error handling, also add surrealdb
- Improve errornames

<!-- generated by git-cliff -->
