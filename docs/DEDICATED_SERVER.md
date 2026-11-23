# The dedicated server

`ss2-server` (T08-020) is the headless server: no window, GPU or audio. This page says what it does **today** and what it does not.

> **There is no network in it yet.** The transport (RFC 0006) does not exist, so the server cannot be joined. It runs the simulation
> loop over an **empty world**, reads its configuration, logs, answers console commands and serves metrics. That is what the task
> delivers; the network is T08-004 and the transport.

## Running it

```
ss2-server --print-config > server.toml        # an annotated example with every key at its default
ss2-server --config server.toml                # run
ss2-server --config server.toml --bind 0.0.0.0:27961 --level Levels/Egypt/Sirius     # the command line wins over the file
```

## The configuration file

TOML; **every key is optional**, and **an unknown key is an error** (a typo such as `max_player = 4` must not silently leave the limit at 8).
All problems with the values are reported at once and the server exits with status 2.

| Key | Default | Meaning |
|---|---|---|
| `name` | `"SS2RS Server"` | 1 to 64 characters |
| `bind` | `"0.0.0.0:27960"` | an **IP address and port**, not a host name |
| `max_players` | `8` | 1 to 64 |
| `password` | none | the session password (1 to 128 bytes); never printed (`Debug` shows `<set>`) |
| `tick_hz` | `30` | 1 to 1000. The original's rate comes from T01-022; 30 is a placeholder |
| `level`, `data_dir` | none | the level and the player's own install |
| `log_dir` | none | write a rotating log file here (`ss2.log`, 5 MiB, 5 old files) as well as to the terminal |
| `metrics_bind` | none | serve metrics here (`127.0.0.1:9100`); **off unless set** |
| `console` | `true` | read commands from standard input |
| `motd` | `""` | a message for joining players, up to 200 characters |

## The console

Type on standard input: `status` (tick, players and uptime), `quit`, `help`, and the console's own commands. An unknown command prints an error
and does not stop the server. In a container without a terminal, standard input is closed and the server simply keeps running.
**Remote administration (an "RCON") is not implemented**: a network console needs authentication and a security review first.

## Metrics

With `metrics_bind` set, a tiny HTTP endpoint serves `GET /metrics` (the Prometheus text format) and `GET /healthz` (`ok`), nothing else
(`404`, `405` for other methods, `400` for garbage, `431` for a request over 8 KiB; a slow client is cut off after 0.5 s). Metrics:
`ss2_server_uptime_seconds`, `ss2_server_tick`, `ss2_server_tick_rate_hz`, `ss2_server_players`, `ss2_server_max_players`,
`ss2_server_tick_duration_microseconds`, `ss2_server_tick_duration_max_microseconds` and `ss2_server_ticks_dropped_total` (ticks the server
gave up when it fell behind, see `docs/SIMLOOP.md`). The endpoint is **plain HTTP without authentication**: keep it on `127.0.0.1` or put
something in front of it.

## Docker

```
docker build -t ss2-server .
docker run --rm -p 27960:27960/udp -p 127.0.0.1:9100:9100 \
  -v "$PWD/server.toml:/config/server.toml:ro" -v "$HOME/ss2:/data:ro" ss2-server --config /config/server.toml
```

**This `Dockerfile` has not been built yet** (see "Not covered"). The image has the server binary and nothing else (a `debian:bookworm-slim` base, a non-root user, volumes `/data` and `/logs`);
**no game data is ever copied into it** (`.dockerignore` excludes it too). In the configuration use `bind = "0.0.0.0:27960"`,
`metrics_bind = "0.0.0.0:9100"` (the port is published only on the host's loopback above), `log_dir = "/logs"`, `data_dir = "/data"`.

## How it was tested

Twenty-nine unit tests: the configuration (defaults, a full file, unknown keys and wrong types named, all problems at once, addresses that must
be IPs, the command line overriding the file, the password hidden in `Debug`, the example file parsing back to the defaults), the metrics (the
text format, the endpoint on a real local socket for every path and method, garbage, an oversize request, a client that never finishes, the
port released on stop), the loop on a **fake clock** (1 second is exactly the tick rate in ticks at 20, 30 and 60 Hz; it sleeps instead of
spinning; `quit` stops it and later lines are not run; `status`; unknown commands; the console switched off; a one-minute stall counted as dropped ticks and
then normal pace; tick times and uptime published), and the command line (`--print-config`, `--config`, missing and broken files). The real
binary was also run once by hand on Windows with a config file, a metrics port, a log directory and a piped `status` and `quit`: it ticked at 29.7 Hz, served
metrics, wrote `ss2.log` and exited 0 on `quit`.

Twelve mutation checks each fail a test (unknown keys accepted, the password shown, the player and address checks removed, the command line
ignored, any method served, no request size limit, the console switch ignored, dropped ticks not counted, a loop that never sleeps, lines run
after `quit`, a wrong health path). **One survived at first** (the size limit: my test accepted any 4xx, and a served 404 is one) and the test
now requires the 431.

## Not covered

- **The network** (T08-004, the transport), so nobody can join; the `bind` address is read and validated but nothing listens on it.
- **Remote administration** and authentication for it; the metrics endpoint has none.
- **Graceful stop on a signal** (SIGTERM from `docker stop`): the process is terminated; `quit` on the console stops it cleanly. A signal handler needs a dependency or `unsafe` and is a follow-up.
- **Log levels from the configuration** (`RUST_LOG` works as for the other binaries), **log shipping**, a **systemd unit**.
- **Linux and macOS**: the code is portable (`std::net`, `std::thread`) but only Windows ran it.
- **The `Dockerfile` has never been built.** Docker Desktop's engine would not start on the author's machine (its API answered 500 for several minutes), so the image is **unverified**: the base image tags were checked to exist, nothing more. The first person with a working Docker should run `docker build -t ss2-server .` and `docker run --rm ss2-server --print-config`.
