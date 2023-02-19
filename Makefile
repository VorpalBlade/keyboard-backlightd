# This makefile exists to allow for an install target, since it seems
# cargo install is too basic to handle installing system services properly.

CARGO_FLAGS ?=
DESTDIR ?=
PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin
ETCDIR ?= $(PREFIX)/etc

CONFDIR ?= $(ETCDIR)/conf.d
SYSTEMDDIR ?= $(PREFIX)/lib/systemd/system

PROG := target/release/keyboard-backlightd

all: $(PROG)

$(PROG):
	# Let cargo figure out if a build is needed
	cargo build --release $(CARGO_FLAGS)

etc/keyboard-backlightd.service: etc/keyboard-backlightd.service.in Makefile
	sed -e "s#{BINDIR}#$(BINDIR)#" -e "s#{CONFDIR}#$(CONFDIR)#" $< > $@

install: etc/keyboard-backlightd.service
	install -d $(DESTDIR)$(BINDIR) $(DESTDIR)$(SYSTEMDDIR) $(DESTDIR)$(CONFDIR)
	install $(PROG) $(DESTDIR)$(BINDIR)
	install -m644 etc/keyboard-backlightd.service $(DESTDIR)$(SYSTEMDDIR)
	install -Dm644 etc/keyboard-backlightd.conf $(DESTDIR)$(CONFDIR)/keyboard-backlightd

.PHONY: all install $(PROG)
