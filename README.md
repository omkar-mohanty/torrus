## Torrus
a bittorrent client focusing on maximum performance.
## Motivation
This is mostly a personal project which I made in my spare time to understand about Networking and distributed systems. 
### File share strategy
Torrus client upon connection with a Peer immediately sends `Interested` message. Most of the time the coresponding remote peer responds with a `Unchoke` message. The client and the remote peer can then start the Bittorrent wire protocol.
## Usage 
```bash 
Usage: torrus <COMMAND>

Commands:
  download  Download from a .torrent file
  list      List all torrents currently in the client
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

