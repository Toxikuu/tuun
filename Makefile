all: build

build:
	cargo build --release

clean:
	cargo clean

install: build
	install -Dm644 ./config.toml /usr/share/tuun/default_config.toml
	install -Dm755 target/release/tuun /usr/libexec/tuun
	install -Dm755 tuun.sh /usr/bin/tuun
	install -Dm755 quu.sh /usr/bin/quu

uninstall:
	rm -rf /usr/share/tuun /tmp/tuun
	rm -f /usr/libexec/tuun /usr/bin/tuun /usr/bin/quu
