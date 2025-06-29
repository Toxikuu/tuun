all: build

build:
	cargo build --release

clean:
	cargo clean

install:
	install -Dm644 ./config.toml       $(DESTDIR)/usr/share/tuun/default_config.toml
	install -Dm755 target/release/tuun $(DESTDIR)/usr/libexec/tuun
	install -Dm755 scripts/tuun.sh     $(DESTDIR)/usr/bin/tuun
	install -Dm755 scripts/quu.sh      $(DESTDIR)/usr/bin/quu
	install -Dm755 scripts/fzm         $(DESTDIR)/usr/bin/fzm

uninstall:
	rm -rf $(DESTDIR)/usr/share/tuun   $(DESTDIR)/tmp/tuun
	rm -f  $(DESTDIR)/usr/libexec/tuun $(DESTDIR)/usr/bin/tuun $(DESTDIR)/usr/bin/quu $(DESTDIR)/usr/bin/fzm
