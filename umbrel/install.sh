#!/usr/bin/env bash

export UMBREL_DIR=$(echo $1 | sed 's:/*$::')

##: We need to know where we are, or else bail
if [ ! -f "4G6CFc9vkBsi.test" ]; then
    echo "You must run this from the: [/umbrel] subfolder of the helipad repo."
    exit 1
fi

##: Sanity check the folder they gave us
if [ ! -f "$UMBREL_DIR/docker-compose.yml" ]; then
    echo "What you gave: [$UMBREL_DIR] doesn't look like the umbrel directory."
    exit 2
fi
if [ ! -d "$UMBREL_DIR/apps" ]; then
    echo "What you gave: [$UMBREL_DIR] doesn't look like the umbrel directory."
    exit 2
fi

##: Looks good so hit it
mkdir -p $UMBREL_DIR/apps/podcasting20-boosts/data
cp docker-compose.yml $UMBREL_DIR/apps/podcasting20-boosts

cd $UMBREL_DIR
./scripts/app install podcasting20-boosts

cd -
