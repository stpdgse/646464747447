#!/bin/bash
# Compile the game with the TeaVM tool directly. usage: run.sh [outdir]
cd "$(dirname "$0")"
export JAVA_HOME=$PWD/../work/jdk25
TOOL=$(ls $PWD/libs/tool/*.jar | tr '\n' ':')
GAME="$PWD/../work/client.jar $PWD/../classlib-ext/build/libs/classlib-ext.jar $(ls $PWD/libs/game/*.jar | tr '\n' ' ') $(ls $PWD/libs/tool/*.jar | tr '\n' ' ')"
exec $JAVA_HOME/bin/java -Xmx11g -Xss512m -cp "classes:$TOOL" Driver "${1:-out}" net.minecraft.client.main.Main $GAME
