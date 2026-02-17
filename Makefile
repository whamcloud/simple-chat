SHELL := /bin/bash

GOBIN ?= $(shell go env GOBIN)
ifeq ($(GOBIN),)
GOBIN := $(shell go env GOPATH)/bin
endif

BIN_DIR := bin
PROTO_DIR := api/proto
GEN_DIR := gen
GEN_CHAT_PB := $(GEN_DIR)/chat/v1/chat.pb.go
COVERAGE_OUT := coverage.out
COVERAGE_HTML := coverage.html
DOC_PORT := 6060
DOC_BASE := http://localhost:$(DOC_PORT)
GOLANGCI_LINT_VERSION := v2.4.0

.PHONY: all gen install build test format format-check coverage covhtml doc clean superclean vet staticcheck tidy install-golangci-lint pre-commit smoke

all: build

install: install-golangci-lint
	@command -v protoc >/dev/null 2>&1 || { \
		if command -v brew >/dev/null 2>&1; then brew install protobuf; \
		elif command -v apt-get >/dev/null 2>&1; then sudo apt-get update && sudo apt-get install -y protobuf-compiler; \
		elif command -v dnf >/dev/null 2>&1; then sudo dnf install -y protobuf-compiler; \
		elif command -v pacman >/dev/null 2>&1; then sudo pacman -Sy --noconfirm protobuf; \
		else echo "ERROR: protoc not found and no supported package manager detected. Install protobuf/protoc manually."; exit 1; \
		fi; \
	}
	@command -v protoc-gen-go >/dev/null 2>&1 || go install google.golang.org/protobuf/cmd/protoc-gen-go@latest
	@command -v protoc-gen-go-grpc >/dev/null 2>&1 || go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@latest
	@command -v staticcheck >/dev/null 2>&1 || go install honnef.co/go/tools/cmd/staticcheck@2025.1.1
	@command -v godoc >/dev/null 2>&1 || go install golang.org/x/tools/cmd/godoc@latest

install-golangci-lint:
	@command -v golangci-lint >/dev/null 2>&1 || { \
		echo "Installing golangci-lint $(GOLANGCI_LINT_VERSION) into $(GOBIN)"; \
		command -v curl >/dev/null 2>&1 || (echo "ERROR: curl is required" && exit 1); \
		mkdir -p $(GOBIN); \
		curl -sSfL https://raw.githubusercontent.com/golangci/golangci-lint/master/install.sh \
		  | sh -s -- -b $(GOBIN) $(GOLANGCI_LINT_VERSION); \
	}

# Keep go.mod and go.sum tidy
tidy:
	@test -f $(GEN_CHAT_PB) || $(MAKE) gen
	go mod tidy

gen: install
	mkdir -p $(GEN_DIR)
	protoc \
		-I $(PROTO_DIR) \
		--go_out=$(GEN_DIR) --go_opt=paths=source_relative \
		--go-grpc_out=$(GEN_DIR) --go-grpc_opt=paths=source_relative \
		$(PROTO_DIR)/chat/v1/chat.proto

build: gen
	mkdir -p $(BIN_DIR)
	go build -o $(BIN_DIR)/chat-server ./cmd/chat-server
	go build -o $(BIN_DIR)/chat-client ./cmd/chat-client

# Format code per standard conventions
format:
	go fmt ./...

format-check:
	@unformatted=$$(gofmt -l $$(find . -type f -name '*.go')); \
	if [ -n "$$unformatted" ]; then \
		echo "Unformatted Go files:"; \
		echo "$$unformatted"; \
		exit 1; \
	fi

# Run go vet
vet:
	go vet ./...

# Run static analysis
staticcheck:
	@command -v staticcheck >/dev/null 2>&1 || $(MAKE) install
	staticcheck ./...

golangci-lint:
	golangci-lint run ./...

# Lint: run all local checks
lint: format-check vet staticcheck golangci-lint

test: lint
	go test -v ./...

smoke: build
	python3 scripts/ci_smoke.py

coverage:
	go test -covermode=atomic -coverpkg=./... -coverprofile=$(COVERAGE_OUT) ./...

covhtml: coverage
	go tool cover -html=$(COVERAGE_OUT) -o $(COVERAGE_HTML)
	@printf "View file://${PWD}/$(COVERAGE_HTML)#file0 \n"

doc: install
	@printf "Godoc shows exported symbols only.\n"
	@printf "Module docs: $(DOC_BASE)/pkg/github.com/arunksaha/simple-chat/\n"
	@printf "Server package docs: $(DOC_BASE)/pkg/github.com/arunksaha/simple-chat/internal/server/\n"
	@printf "Server source view: $(DOC_BASE)/src/github.com/arunksaha/simple-chat/internal/server/server.go\n"
	@printf "CTRL-C when done.\n"
	godoc -http=:$(DOC_PORT)

clean:
	rm -rf $(BIN_DIR)/* $(COVERAGE_OUT) $(COVERAGE_HTML)

superclean: clean
	rm -rf $(GEN_DIR)/*
