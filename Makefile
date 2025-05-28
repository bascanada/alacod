PROFILE ?= dev

NUMBER_PLAYER ?= 2

CURRENT_TAG := $(shell git describe --tags --exact-match HEAD 2>/dev/null)


LOG_DIR := ./logs
LOG_PREFFIX := game_run
FILTERED_LOG_DIR := ./logs/filtered
GREP_FILTER := 'ggrs{'


ifeq ($(CURRENT_TAG),)
	LATEST_TAG := $(shell git describe --tags --abbrev=0 2>/dev/null || echo "v0.0.0")
    SHORT_SHA := $(shell git rev-parse --short HEAD)
	VERSION := $(LATEST_TAG)-$(SHORT_SHA)
else
	VERSION := $(CURRENT_TAG)
endif


ifeq ($(PROFILE), dev)
	export MODE_DIR := debug
	export CARGO_TARGET_DIR := ./target
endif

ifeq ($(PROFILE), prod)
	export MODE_DIR := release
	export RELEASE := --release

endif

ifeq ($(TARGET), native)
	export CARGO_TARGET_DIR := ./target
endif


ifeq ($(TARGET), web)
	export RUSTFLAGS := --cfg=web_sys_unstable_apis
	export CARGO_TARGET_DIR := ./target_wasm
endif




# ALL

all: test format


# Misc

clean:
	@echo "Cleaning the project..."
	cargo clean

format:
	@echo "Running fmy..."
	cargo fmt --all -- --emit=files


# Test

test:
	@echo "Running tests with profile"
	cargo test


# Env


# Dependencies

dep_web:
	rustup target add wasm32-unknown-unknown
	cargo install wasm-bindgen-cli

dep: dep_web

# Dev run

map_preview:
	cargo run --example map_preview $(ARGS) --features native

map_generation:
	cargo run --example map_generation $(ARGS)

map_explorer:
	cargo run --example map_explorer $(ARGS) --features native


character_tester:
	APP_VERSION=$(VERSION) cargo run --example character_tester $(ARGS) --features native -- --local-port 7000 --players localhost

character_tester_matchbox:
	APP_VERSION=$(VERSION) cargo run --example character_tester $(ARGS) --features native -- --number-player $(NUMBER_PLAYER) --matchbox "wss://matchbox.bascanada.org" --lobby test_2 --players localhost remote --cid $(CID)

host_website:
	cd website && APP_VERSION=$(VERSION) npm run dev

cp_asset:
	mkdir -p ./website/static/$(VERSION)/assets/
	cp -r ./assets/* ./website/static/$(VERSION)/assets/

build_map_preview_web:
	APP_VERSION=$(VERSION) cargo build --example map_preview --target wasm32-unknown-unknown --features bevy_ecs_tilemap/atlas $(RELEASE)
	wasm-bindgen --out-dir ./website/static/$(VERSION)/map_preview --out-name wasm --target web $(CARGO_TARGET_DIR)/wasm32-unknown-unknown/$(MODE_DIR)/examples/map_preview.wasm

build_character_tester_web:
	APP_VERSION=$(VERSION) cargo build --example character_tester --target wasm32-unknown-unknown $(RELEASE)
	wasm-bindgen --out-dir ./website/static/$(VERSION)/character_tester --out-name wasm --target web $(CARGO_TARGET_DIR)/wasm32-unknown-unknown/$(MODE_DIR)/examples/character_tester.wasm

build_wasm_apps: cp_asset build_map_preview_web build_character_tester_web

build_website: build_wasm_apps
	cd website && npm ci && APP_VERSION=$(VERSION) npm run build

build_docker_website: build_wasm_apps
	docker build --build-arg APP_VERSION=$(VERSION) -f ./website/Dockerfile ./website -t ghcr.io/bascanada/alacod:latest


# CID_1 , CID_2
diff_run:
	grep 'system="ggrs_' game_run_bob.log > ggrs_bob_filtered.log
	

# Publish
push_docker_website:
	docker push ghcr.io/bascanada/alacod:latest


print_version:
	@echo "Current Tag: $(CURRENT_TAG)"
	@echo "Version: $(VERSION)"


diff_log:
	mkdir -p $(FILTERED_LOG_DIR)
	cat $(LOG_DIR)/$(LOG_PREFFIX)_$(CID_1).log | grep $(GREP_FILTER) > $(FILTERED_LOG_DIR)/$(CID_1).log
	cat $(LOG_DIR)/$(LOG_PREFFIX)_$(CID_2).log | grep $(GREP_FILTER) > $(FILTERED_LOG_DIR)/$(CID_2).log
	diff $(FILTERED_LOG_DIR)/$(CID_1).log $(FILTERED_LOG_DIR)/$(CID_2).log