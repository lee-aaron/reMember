# Proxy DLL to do client modifications

## Building

Either use cross(https://github.com/cross-rs/cross) or build this library locally on a windows machine. I'd recommend setting up a Windows 7/10 VM and use SSH to develop on It remotely.

## Usage instructions

Move the build `dinput8.dll` from the `target/release` directory to your Maple Story directory(same directory as your localhost .exe). When you launch the localhost you should see a console being opened.

## Features

* Logging Packet structure tracing data
* Dump the whole string pool
* Catch exceptions before the process is about to crash
* Skip the logo

## TODO

* Add key dumping feature
* Config
* Auto Update via the launcher
