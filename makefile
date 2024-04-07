service_active:=$(shell systemctl is-active NS50MU-fan-controller.service)

compile:
	cargo build --release

install-bin: compile
	sudo chmod +x ./target/release/ns50mu_fan_controller_rs
ifeq ($(service_active) , active)
	sudo systemctl stop NS50MU-fan-controller
endif
	sudo cp ./target/release/ns50mu_fan_controller_rs /usr/local/bin/
ifeq ($(service_active) , active)
	sudo systemctl start NS50MU-fan-controller
endif

install-service: install-bin
	sudo cp NS50MU-fan-controller.service /etc/systemd/system/
	sudo systemctl enable NS50MU-fan-controller.service

all: install-service
	sudo systemctl start NS50MU-fan-controller

clean:
	rm -rf ./target

uninstall:
	sudo systemctl stop NS50MU-fan-controller
	sudo systemctl disable NS50MU-fan-controller.service
	sudo rm /etc/systemd/system/NS50MU-fan-controller.service
	sudo rm /usr/local/bin/ns50mu_fan_controller_rs
