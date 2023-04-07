#!/bin/sh
cargo run --bin encode --features std,postcard "$1" | curl --request POST --data-binary @- http://localhost:3030/backdoor
