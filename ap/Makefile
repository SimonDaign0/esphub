DIR = $(CURDIR)
GATEWAY="10.0.2.1"
PORT="8080"
HOSTNAME="Esphub"
PASSWORD="password"

.PHONY: run
run:
	HOSTNAME=$(HOSTNAME) PASSWORD=$(PASSWORD) GATEWAY=$(GATEWAY) PORT=$(PORT) cargo run --release

b:
	cargo build --release
