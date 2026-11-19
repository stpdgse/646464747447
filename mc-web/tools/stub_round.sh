#!/bin/bash
# One placeholder round: add newly reported classes, enrich with true hierarchies, rebuild jar, recompile.
cd "$(dirname "$0")/.."
export JAVA_HOME=$PWD/work/jdk25
S=classlib-ext/src/main/resources/mcweb/stub-classes.txt
python3 -I tools/genstubs.py driver/run.log $S >/dev/null
CP="driver/stubinfo-classes:work/client.jar:$(ls driver/libs/game/*.jar driver/libs/removed/*.jar | tr '\n' ':')"
grep -E '^(class|interface|exception) ' $S | awk '{print $2}' | $JAVA_HOME/bin/java -cp "$CP" StubInfo 2>/dev/null > driver/stubinfo.txt
python3 -I tools/enrich_stubs.py $S driver/stubinfo.txt >/dev/null
(cd classlib-ext && JAVA_HOME=/usr/lib/jvm/java-21-openjdk-amd64 ../work/gradle-9.2.1/bin/gradle --no-daemon -q jar 2>&1 | grep -A2 error)
(cd driver && ./run.sh out > run.log 2>&1)
grep -E 'GENERAT' driver/run.log | head -1
grep -E '^PROBLEM:' driver/run.log | cut -c1-120 | sort | uniq -c | sort -rn | head -3
