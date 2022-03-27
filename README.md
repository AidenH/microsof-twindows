# microsof-twindows

the 't' is short for 'tiling'.

currently usable but somewhat rough around the edges.

mtwm is a quasi-manual tiling window manager for the X window system written with rust 
xcb bindings. it features a nudge function which allows one to stretch smaller windows 
to the edges of the screen using vim binds alt-hjkl and then revert nudged windows to 
previous dimensions using alt-r.

you may want to edit keybinds for your own personal configuration.
they can be found under the KEYBINDS heading near the bottom of main.rs.

mtwm also allows for a startup script, located at ~/.microsof-twindows/autostart.sh.
mine currently runs a ".fehbg &" and a simple dzen2 status bar script.
don't forget to make your autostart executable :)
