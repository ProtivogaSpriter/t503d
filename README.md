# what
a pseudodriver (i.e. not an actual driver, just a program which pretends to be one) for the 10moons t503 pad.  
runs in the background, can be launched automatically via init systems, allows for multiple connected at once, has a config.  

# how
run the binary as root (with sudo). it will daemonize and run in the background.  
if you want it to run at boot time, use the systemd.install script. (or write a service for your own init system yourself)  
config file will generate automatically in /etc/t503d/config.yaml at start whenever the config is missing.  
the keycodes for the buttons are customizable in the config and can be anything a keyboard can deliver;  
see [the kernel event codes](https://github.com/torvalds/linux/blob/master/include/uapi/linux/input-event-codes.h) page for all available. (note: only ones prefixed with "KEY" or "BTN" work.)  
you can also use keycombos by declaring them in the config like so: KEY\_A+KEY\_LEFTSHIFT  

# who
the concept for the pseudodriver isn't really mine originally, it was executed before by:  
[alex-s-v](https://github.com/alex-s-v/10moons-driver), who made the OG, and  
[calico-cat-3333](https://github.com/calico-cat-3333/10moons-t503-driver), who made a slight upgrade.  
i basically took the code from calico-cat-3333 and adapted it to rust, adding daemon, multithreading and fixing the weird way the tablet behaves depending on when it's connected.  

> done for a bar of chocolate in 4 days time
