#what
a pseudodriver (i.e. not an actual driver, just a program which pretends to be one) for the 10moons t503 pad.
runs in the background, allows for multiple connected at once, unintrusive, has a config.

#how
run the binary as root (with sudo). it will daemonize and run in the background.
if you want it to run at boot time, use the systemd.install script.
config file will generate automatically in /etc/t503d/config.yaml at start whenever the config is missing.
openrc support coming never lol

#who
the concept for the pseudodriver isn't really mine originally, it was executed before by:
[alex-s-v]<https://github.com/alex-s-v/10moons-driver>, who made the OG, and
[calico-cat-3333]<https://github.com/calico-cat-3333/10moons-t503-driver>, who made a slight upgrade.
i basically took the code from calico-cat-3333 and adapted it to evdev and rust.

> done for a bar of chocolate in 4 days time
