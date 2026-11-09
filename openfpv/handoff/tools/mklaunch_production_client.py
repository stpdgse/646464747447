import json, os, sys, urllib.request, zipfile, shutil
root = sys.argv[1]
fv = json.load(open(f"{root}/versions/1.12.2-forge-14.23.5.2860/1.12.2-forge-14.23.5.2860.json"))
vv = json.load(open(f"{root}/versions/1.12.2/1.12.2.json"))
libs = vv["libraries"] + fv["libraries"]
cp, natives = [], []
def allowed(l):
    r = l.get("rules")
    if not r: return True
    ok = False
    for rule in r:
        osn = rule.get("os", {}).get("name")
        if osn is None or osn == "linux":
            ok = rule["action"] == "allow"
    return ok
def fetch(path, url):
    dst = f"{root}/libraries/{path}"
    if not os.path.exists(dst):
        os.makedirs(os.path.dirname(dst), exist_ok=True)
        print("downloading", path)
        urllib.request.urlretrieve(url, dst)
    return dst
for l in libs:
    if not allowed(l): continue
    d = l.get("downloads", {})
    art = d.get("artifact")
    if art and art.get("url") != "":
        cp.append(fetch(art["path"], art["url"]))
    elif art:  # forge's own jars are installed by the installer already
        p = f"{root}/libraries/{art['path']}"
        if os.path.exists(p): cp.append(p)
    elif "name" in l and not d:
        g, a, v = l["name"].split(":")[:3]
        p = f"{root}/libraries/{g.replace('.','/')}/{a}/{v}/{a}-{v}.jar"
        if os.path.exists(p): cp.append(p)
    cls = d.get("classifiers", {})
    nat = l.get("natives", {}).get("linux")
    if nat and nat in cls:
        natives.append(fetch(cls[nat]["path"], cls[nat]["url"]))
nd = f"{root}/natives"
shutil.rmtree(nd, ignore_errors=True); os.makedirs(nd)
for n in natives:
    with zipfile.ZipFile(n) as z:
        for m in z.namelist():
            if m.endswith(".so"): z.extract(m, nd)
cp.append(f"{root}/versions/1.12.2/1.12.2.jar")
seen, out = set(), []
for c in cp:
    if c not in seen: seen.add(c); out.append(c)
open(f"{root}/classpath.txt", "w").write(":".join(out))
print(len(out), "classpath entries;", len(natives), "native jars")
print("mainClass", fv["mainClass"])
print("args", fv.get("minecraftArguments"))
